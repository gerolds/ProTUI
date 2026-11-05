# ADR: Makes each missing <out>/<kind>/<key>.png from a prompts TOML with the Codex CLI's image tool, six at a time, and shrinks it to at most --width px wide; never overwrites an existing picture.
# ADR: An entry is { style, subject } or a variant { style, from, change } that edits the picture of `from` in the same kind; variants wait for the wave that makes their base. The prompts file is checked strictly before any run.
# ADR: Codex comes from the CODEX_EXE environment variable, else `codex` on PATH — no version-specific install path.
import argparse
import concurrent.futures as cf
import os
import re
import shutil
import subprocess
import sys
import tomllib

REPO = os.path.normpath(os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", ".."))
DEFAULT_PROMPTS = os.path.join(REPO, "assets", "pictures", "prompts.toml")
JOBS = 6
TIMEOUT_S = 1200
NAME = re.compile(r"[a-z0-9-]+")


class PromptError(Exception):
    pass


def parse_args():
    parser = argparse.ArgumentParser(
        description="Make each missing picture named in a prompts file with the Codex CLI's image tool.",
        epilog="Codex: set CODEX_EXE to the executable, or put `codex` on PATH. Needs Python 3.11+ and Pillow. "
        "Delete a picture to make it again.",
    )
    parser.add_argument("prompts", nargs="?", default=DEFAULT_PROMPTS,
                        help="prompts TOML file (default: assets/pictures/prompts.toml in the repository)")
    parser.add_argument("--out", help="output folder; pictures go to <out>/<kind>/<key>.png "
                        "(default: the prompts file's folder)")
    parser.add_argument("--width", type=int, default=256, help="largest stored width in pixels (default: 256)")
    args = parser.parse_args()
    if args.width < 1:
        parser.error("--width must be at least 1")
    return args


def load_prompts(file):
    """Returns (styles, entries) or raises PromptError naming every problem."""
    try:
        with open(file, "rb") as f:
            data = tomllib.load(f)
    except OSError as error:
        raise PromptError(f"{file}: cannot read: {error.strerror}") from error
    except tomllib.TOMLDecodeError as error:
        raise PromptError(f"{file}: {error}") from error
    problems = []
    styles = data.pop("styles", None)
    if not isinstance(styles, dict) or not styles:
        problems.append(f"{file}: styles: a [styles] table with at least one style is required")
        styles = {}
    for name, text in styles.items():
        if not isinstance(text, str) or not text.strip():
            problems.append(f"{file}: styles.{name}: must be a non-empty string")
    entries = []
    for kind, table in data.items():
        if not isinstance(table, dict):
            problems.append(f"{file}: {kind}: must be a table of entries")
            continue
        if not NAME.fullmatch(kind):
            problems.append(f"{file}: {kind}: a kind must be lowercase letters, digits and '-'")
        for key, entry in table.items():
            where = f"{file}: {kind}.{key}"
            if not NAME.fullmatch(key):
                problems.append(f"{where}: a key must be lowercase letters, digits and '-'")
            if not isinstance(entry, dict):
                problems.append(f"{where}: must be an inline table")
                continue
            fields = set(entry)
            if fields not in ({"style", "subject"}, {"style", "from", "change"}):
                problems.append(f"{where}: keys must be {{style, subject}} or {{style, from, change}}, found {sorted(fields)}")
                continue
            bad = [f for f in sorted(fields) if not isinstance(entry[f], str) or not entry[f].strip()]
            for field in bad:
                problems.append(f"{where}: {field}: must be a non-empty string")
            if bad:
                continue
            if entry["style"] not in styles:
                problems.append(f"{where}: style: no style {entry['style']!r} in [styles]")
            if "from" in entry:
                base = table.get(entry["from"])
                if base is None or entry["from"] == key:
                    problems.append(f"{where}: from: names no other entry in [{kind}]")
                elif not isinstance(base, dict) or "from" in base:
                    problems.append(f"{where}: from: must name a plain {{style, subject}} entry")
            entries.append((kind, key, entry))
    if problems:
        raise PromptError("\n".join(problems))
    return styles, entries


def find_codex():
    exe = os.environ.get("CODEX_EXE")
    if exe:
        if not os.path.isfile(exe):
            raise PromptError(f"CODEX_EXE: {exe}: no such file")
        return exe
    exe = shutil.which("codex")
    if exe is None:
        raise PromptError("codex: not found; set CODEX_EXE to the Codex CLI executable or put `codex` on PATH")
    return exe


def picture(out, kind, key):
    return os.path.join(out, kind, key + ".png")


def generate(codex, out, width, kind, key, entry, style):
    from PIL import Image

    folder = os.path.join(out, kind)
    os.makedirs(folder, exist_ok=True)
    save = f"Then copy the generated PNG into ./{key}.png with a shell command, and reply with only the final path."
    extra = []  # `--image=<file>`: a bare `-i <file>` would also take the prompt as a second image
    if "from" in entry:
        extra = [f"--image={picture(out, kind, entry['from'])}"]
        prompt = (
            "Use your image generation tool to edit the attached picture into ONE new image. Keep everything the same: "
            "the framing, the camera, the composition, the subject's identity and pose, the light, the palette and the "
            f"painting style. Change only this: {entry['change']} Style reminder: {style}\n{save}"
        )
    else:
        prompt = f"Use your image generation tool to create ONE image. {entry['subject']} {style}\n{save}"
    try:
        result = subprocess.run(
            [codex, "exec", "--skip-git-repo-check", "--ephemeral", "-s", "workspace-write", "--color", "never",
             "-C", folder, *extra, prompt],
            capture_output=True, text=True, timeout=TIMEOUT_S, encoding="utf-8", errors="replace",
        )
        tail = (result.stderr or "")[-300:]
    except subprocess.TimeoutExpired:
        tail = f"timed out after {TIMEOUT_S} s"
    path = picture(out, kind, key)
    if not os.path.exists(path):
        return f"{kind}/{key}", tail or "no picture written"
    with Image.open(path) as image:
        image.load()
    if image.width > width:
        image.resize((width, round(image.height * width / image.width)), Image.LANCZOS).save(path)
    return f"{kind}/{key}", "ok"


def main():
    args = parse_args()
    out = args.out or os.path.dirname(os.path.abspath(args.prompts))
    try:
        styles, entries = load_prompts(args.prompts)
        missing = [e for e in entries if not os.path.exists(picture(out, e[0], e[1]))]
        if not missing:
            print("nothing to make: every picture exists")
            return 0
        codex = find_codex()
        import PIL  # noqa: F401  (fail before spending a Codex run)
    except PromptError as error:
        print(error, file=sys.stderr)
        return 2
    except ImportError:
        print("Pillow: not installed; run `python -m pip install Pillow`", file=sys.stderr)
        return 2
    # Each wave runs every missing picture whose base exists; a wave that makes nothing new ends the run.
    while True:
        ready = [
            (kind, key, entry, styles[entry["style"]])
            for kind, key, entry in entries
            if not os.path.exists(picture(out, kind, key))
            and ("from" not in entry or os.path.exists(picture(out, kind, entry["from"])))
        ]
        if not ready:
            break
        with cf.ThreadPoolExecutor(JOBS) as pool:
            results = [job.result() for job in [pool.submit(generate, codex, out, args.width, *r) for r in ready]]
        for result in results:
            print(*result)
        if not any(status == "ok" for _, status in results):
            break
    left = [f"{kind}/{key}" for kind, key, _ in entries if not os.path.exists(picture(out, kind, key))]
    for name in left:
        print(name, "missing")
    return 1 if left else 0


if __name__ == "__main__":
    sys.exit(main())
