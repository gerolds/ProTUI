# Treatment

The user's words from the first session (2026-11-05), recorded verbatim.

---

this prototype will be a game about excavation and going on expeditions to extract artefacts/relics/salvage. The real game is a voxel digging game, it has a number of action-features that are proven to be fun. What this prototype aims to figure out is what the action gameplay amounts to. its progress, narrative, upgrades, economy, and motivational loop outside the core. This prototype will abstract the action touchpoints away into progress bars, slotted, branching timelines with event slots, dice rolls and steering choices offered while the player watches the progress happening. Note that this is not a management game. It is a solo experience around multiple expeditions into relatively small 3D levels, zero-g, presumed to be asteroids, walnut like structure, hard shell -> the expedition drop-pod pierces that shell and players go from there. Tools they have:
- digging tool (melts away voxels based on hardness at rate R)
- cutting tool for separating large objects by melting connectors/seams (based on energy transfer and heat capacity) (Hardspace Shipbreaker)
- grapple tool that can telekinetically move objects at short range, power coupled to the zero-g suit's thurst. objects can also be stuck and pryed free.
- tether tool that can be used to tie objects together and to pull them towards an attachment point. (Hardspace Shipbreaker)
- marking tool that allows placement of objects in the world, a wheelbarrow, a skip, a refinery, a beacon, a decal/sticker, extraction flare etc. really anything to communicat with the rest of the team or game that something should happen.
- scanner tool for identifying objects
- scanner tool for probing the voxel substrate (ultrasound/radar/seismic etc. inspired)
The game world has
- Fully featured 3D Voxel engine that can declare infinite worlds from authored static generator function and stamp programs (pseudo metric noise, SDF shapes, scatter of sub-stamps) -> basically how No Man Sky and Minecraft work
- Hostile system, i.e. actor that respond to player proximity by triggering growth, snare, AoI effects (poison, shockwave, etc.), hostiles are thought of as flora or spawner anchored fauna that gives only rudimentary chase without pathfinding.
- Gravity fields, arbitrarily placeable, point, plane, spherical surface etc.
- Drag fields, arbitrarily placeable, can be turbulent, directed, spline-saped. Allows for placing currents and physically simulated convection of objects thrown into drag fields. can also slow/accelerate movement. (Subnautica streams)
- Kinematic/Sim-Physics hybrid, everything besides voxels can be a rigidbody or kinematically simulated object
- Fairly slow paced movement (10m/s max, 10s acceleration to max speed)
- Terminal system by which a player can interact with objects in the field, as shop, or narrative terminal, or as stand-in for a NPC/Dialogue system. Terminals can be arbitrarily complex.
- Source code for QuestSystem (PactSystem) and the menu system can be provided to work on known vocabulary.
- Digging can spawn objects/effects from a loot table.
- Interaction system that can be used to make any object interactable in various ways, from item pickup, switch activation, terminal interaction and really anything else that maps to "press E to interact"
- Basic player inventory and container interaction. Shops can be modelled as a container with a currency transfer per object or as a menu system.
- All progression state is represented as token counts (Unity ScriptableObjects, type objects) in abstract IInventory components. Every entity/actor can have "stats" and "modifierts" by adding an inventory. We can abstract basically everything into that to stay compatible wth the actual game here.
The game overall has a fully fledged guidance system that can represent nearly all commonly encountered quest/missions/knowledge/unlock structures in a Action Adventure or RPG

I may have forgotten a few parts. In any case. The goal here is now to (a) prototype the part of the game that happens before and between expeditions (b) an abstraction over how a expedition plays out so we can figure out what makes decisions during the expedition interesing.
