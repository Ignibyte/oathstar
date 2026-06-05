export const directionAliases = {
  n: "north",
  north: "north",
  s: "south",
  south: "south",
  e: "east",
  east: "east",
  w: "west",
  west: "west",
  u: "up",
  up: "up",
  d: "down",
  down: "down"
};

export const directionLabels = {
  north: "North",
  south: "South",
  east: "East",
  west: "West",
  up: "Up",
  down: "Down"
};

export const world = {
  startRoom: "starfallGate",
  rooms: {
    starfallGate: {
      id: "starfallGate",
      name: "Starfall Gate",
      shortName: "Gate",
      zone: "Gatehouse",
      x: 1,
      y: 3,
      safe: true,
      description:
        "A fallen gate leans under a quiet sky. Silver dust gathers in the cracks, and an unlit lantern hangs from a hook.",
      firstVisit:
        "The old road ends at a gate that remembers a city better than the city remembers itself.",
      exits: {
        north: "oathHall",
        east: "ashOrchard",
        west: "glassReliquary"
      },
      items: ["blackLantern"]
    },
    oathHall: {
      id: "oathHall",
      name: "Oath Hall",
      shortName: "Hall",
      zone: "Civic Ruin",
      x: 1,
      y: 2,
      safe: true,
      description:
        "Broken pews face a dais of green stone. The air tastes of rain even though the ceiling is gone.",
      firstVisit:
        "A figure in a bronze mask turns as you enter, patient as a locked door.",
      exits: {
        south: "starfallGate",
        east: "deepWell",
        north: "ironSanctum"
      },
      npcs: ["maskedWarden"],
      items: []
    },
    ashOrchard: {
      id: "ashOrchard",
      name: "Ash Orchard",
      shortName: "Orchard",
      zone: "Outer Ward",
      x: 2,
      y: 3,
      description:
        "White trees stand in neat rows. Their leaves are ash, their fruit hollow bells, their shadows too long.",
      exits: {
        west: "starfallGate",
        north: "deepWell"
      },
      items: ["sealedLetter"]
    },
    glassReliquary: {
      id: "glassReliquary",
      name: "Glass Reliquary",
      shortName: "Reliquary",
      zone: "Archive",
      x: 0,
      y: 3,
      description:
        "Shelves of cracked glass hold oath tokens, names scratched on metal, and one mirror that refuses your reflection.",
      exits: {
        east: "starfallGate"
      },
      npcs: ["glassArchivist"],
      items: []
    },
    deepWell: {
      id: "deepWell",
      name: "Deep Well",
      shortName: "Well",
      zone: "Civic Ruin",
      x: 2,
      y: 2,
      description:
        "The well descends through black masonry. Something faceted waits below the waterline where ordinary light cannot reach.",
      exits: {
        west: "oathHall",
        south: "ashOrchard"
      },
      items: []
    },
    ironSanctum: {
      id: "ironSanctum",
      name: "Iron Sanctum",
      shortName: "Sanctum",
      zone: "High Ward",
      x: 1,
      y: 1,
      description:
        "Iron ribs arc overhead like a giant cage. A ragged shade kneels before the northern stair, clutching a coal-bright shard.",
      exits: {
        south: "oathHall",
        north: "observatory"
      },
      enemies: ["oathlessShade"],
      items: []
    },
    observatory: {
      id: "observatory",
      name: "Observatory of Dust",
      shortName: "Oathstar",
      zone: "High Ward",
      x: 1,
      y: 0,
      description:
        "A roofless chamber surrounds a brass armillary. At its center rests the Oathstar: a dead little sun with room for three vows.",
      exits: {
        south: "ironSanctum"
      },
      items: []
    }
  },
  items: {
    blackLantern: {
      id: "blackLantern",
      name: "black lantern",
      aliases: ["lantern", "black lantern", "light"],
      takeable: true,
      description:
        "A hooded lantern with black glass. It gives no warmth, but it catches truths ordinary flame misses."
    },
    sealedLetter: {
      id: "sealedLetter",
      name: "sealed letter",
      aliases: ["letter", "sealed letter", "charter"],
      takeable: true,
      description:
        "A brittle letter sealed with green wax. The address is written to the archivist of the west reliquary."
    },
    mercyBell: {
      id: "mercyBell",
      name: "mercy bell",
      aliases: ["bell", "mercy bell"],
      takeable: true,
      description:
        "A thumb-sized bronze bell. Its clapper is wrapped in red thread, as if someone feared what it might forgive."
    },
    brassPrism: {
      id: "brassPrism",
      name: "brass prism",
      aliases: ["prism", "brass prism", "memory shard", "memory"],
      takeable: true,
      description:
        "A prism of brass and old glass. Hold it to the dark and it remembers dawn."
    },
    mercyShard: {
      id: "mercyShard",
      name: "mercy shard",
      aliases: ["mercy", "mercy shard", "glass shard"],
      takeable: true,
      description:
        "A soft green shard from the reliquary mirror. It hums when you stop trying to deserve it."
    },
    emberShard: {
      id: "emberShard",
      name: "ember shard",
      aliases: ["ember", "ember shard", "flame shard", "coal"],
      takeable: true,
      description:
        "A red shard, hot without burning. It keeps the shape of the hand that refused to let go."
    }
  },
  npcs: {
    maskedWarden: {
      id: "maskedWarden",
      name: "masked warden",
      aliases: ["warden", "masked warden", "keeper"],
      description:
        "The warden wears a bronze mask and a robe patched with names. Their hands are empty, deliberately so."
    },
    glassArchivist: {
      id: "glassArchivist",
      name: "glass archivist",
      aliases: ["archivist", "glass archivist", "mirror"],
      description:
        "A person-shaped shimmer stands inside the broken mirror, sorting reflections by the lives they almost had."
    }
  },
  enemies: {
    oathlessShade: {
      id: "oathlessShade",
      name: "oathless shade",
      aliases: ["shade", "oathless shade", "enemy"],
      maxHp: 15,
      attack: 4,
      description:
        "A torn silhouette with a coal of borrowed flame in its chest. It bars the stair north."
    }
  }
};

export const questDefinitions = [
  {
    id: "speakWarden",
    label: "Receive the warden's bell",
    complete: (state) => Boolean(state.flags.talkedToWarden)
  },
  {
    id: "findMemory",
    label: "Lift memory from the deep well",
    complete: (state) => state.flags.oathstarLit || state.inventory.includes("brassPrism")
  },
  {
    id: "quietReliquary",
    label: "Quiet the glass reliquary",
    complete: (state) => state.flags.oathstarLit || state.flags.reliquaryQuieted || state.inventory.includes("mercyShard")
  },
  {
    id: "claimFlame",
    label: "Claim flame from the sanctum",
    complete: (state) => state.flags.oathstarLit || state.inventory.includes("emberShard")
  },
  {
    id: "relightStar",
    label: "Relight the Oathstar",
    complete: (state) => Boolean(state.flags.oathstarLit)
  }
];
