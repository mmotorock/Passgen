"""Builds Passgen's themed wordlist: common, concrete, easy-to-spell words."""
import re
import sys

CATEGORIES = {
    "fruit": """
        apple apricot avocado banana berry cherry coconut date fig grape guava kiwi lemon lime
        lychee mango melon olive orange papaya peach pear plum pomelo quince raisin kumquat
        currant mandarin plantain prune durian rhubarb cranberry blueberry
    """,
    "vegetables": """
        carrot potato tomato onion garlic pepper cabbage lettuce spinach celery cucumber pumpkin
        squash radish turnip beet bean pea corn leek kale broccoli zucchini eggplant parsnip yam
        okra chive ginger sprout mushroom shallot fennel lentil chickpea soybean
    """,
    "food": """
        bread butter cheese cookie cake muffin bagel waffle pancake pizza pasta noodle rice soup
        salad sandwich burger taco burrito omelet toast cereal yogurt honey jam jelly pickle
        popcorn pretzel donut cracker candy toffee fudge brownie cupcake pie tart pudding custard
        biscuit scone crepe dumpling sushi curry stew chili sauce gravy mustard ketchup salsa
        sugar salt flour oatmeal granola almond peanut walnut cashew pecan hazelnut chestnut egg
        bacon ham sausage salami steak lasagna ravioli risotto paella falafel hummus kebab nacho
        tortilla caramel cocoa vanilla cinnamon nutmeg paprika oregano basil parsley thyme mint
        sage clove saffron meatball cheddar marmalade lollipop
    """,
    "drinks": """
        coffee tea juice milk water soda lemonade cider smoothie latte mocha espresso syrup
        milkshake
    """,
    "mammals": """
        cat dog horse cow pig sheep goat rabbit mouse rat hamster squirrel fox wolf bear lion
        tiger leopard cheetah jaguar panther cougar lynx zebra giraffe hippo rhino elephant camel
        llama alpaca donkey mule moose deer elk bison buffalo yak koala kangaroo wombat panda
        otter beaver badger weasel ferret skunk raccoon possum hedgehog mole bat monkey gorilla
        chimp lemur sloth walrus seal dolphin whale orca narwhal hyena jackal coyote meerkat
        gazelle antelope reindeer mammoth hare lamb puppy kitten calf foal pony stallion bull ox
        boar chipmunk mongoose platypus
    """,
    "birds": """
        eagle hawk falcon owl parrot penguin pelican flamingo robin sparrow crow raven pigeon dove
        swan goose duck chicken turkey ostrich emu peacock heron crane stork toucan finch canary
        magpie seagull puffin vulture condor wren lark quail pheasant gull jay kestrel osprey
        cuckoo starling swallow bluebird hen rooster
    """,
    "sea": """
        salmon trout tuna cod shark octopus squid crab lobster shrimp oyster clam mussel scallop
        starfish eel stingray seahorse goldfish carp pike perch bass herring sardine anchovy
        halibut marlin piranha coral plankton urchin turtle tortoise
    """,
    "reptiles_and_bugs": """
        snake python cobra viper lizard gecko iguana frog toad newt dinosaur tadpole ant bee wasp
        hornet beetle moth fly mosquito cricket ladybug firefly spider scorpion termite worm
        cicada mantis snail slug
    """,
    "fantasy": """
        dragon unicorn phoenix griffin wizard robot giant goblin troll fairy mermaid pirate knight
        ninja viking elf dwarf ghost monster genie
    """,
    "household": """
        chair table sofa couch bed lamp mirror pillow blanket carpet rug curtain shelf drawer
        closet cabinet desk stool bench cushion candle clock vase frame poster window door gate
        fence roof wall floor ceiling ladder kettle teapot toaster oven stove fridge freezer sink
        faucet bathtub shower towel soap sponge comb brush razor bucket broom mop basket bottle
        jar cup mug glass plate bowl spoon fork knife spatula ladle whisk pan pot skillet tray
        napkin apron blender mixer grater sieve funnel colander jug pitcher thermos lunchbox
        cooler doormat hanger keyring
    """,
    "office": """
        pencil pen crayon marker eraser ruler notebook paper book folder envelope stamp letter
        scissors stapler tape glue chalk globe map atlas calendar laptop tablet phone camera
        radio speaker headset keyboard monitor printer cable battery charger remote lantern torch
        compass magnet postcard
    """,
    "tools": """
        hammer wrench drill saw axe shovel rake hoe pliers screw nail bolt chisel rope chain hose
        anvil crowbar mallet trowel toolbox pickaxe
    """,
    "clothing": """
        shirt jacket coat sweater hoodie scarf glove mitten hat cap beanie helmet boot shoe sandal
        sneaker slipper sock belt tie vest dress skirt robe gown kimono poncho tuxedo uniform
        backpack purse wallet umbrella watch ring necklace bracelet earring crown tiara button
        zipper pocket collar sleeve
    """,
    "vehicles": """
        car bus truck van taxi train tram subway bicycle bike scooter boat ship canoe kayak raft
        yacht ferry airplane plane jet rocket tractor wagon cart sled sleigh wheel engine anchor
        sail paddle oar tire
    """,
    "nature": """
        tree forest river lake ocean sea beach island mountain hill valley canyon desert meadow
        field garden jungle cave cliff volcano glacier iceberg pond stream creek swamp marsh
        prairie tundra rock stone pebble boulder sand mud soil leaf branch root seed acorn
        pinecone flower petal blossom rose tulip daisy lily orchid lotus poppy violet daffodil
        lavender cactus fern moss ivy bamboo clover grass oak pine maple birch willow cedar palm
        elm spruce aspen redwood sequoia
    """,
    "sky": """
        sun moon star planet comet meteor galaxy cloud rain snow storm thunder rainbow wind breeze
        frost fog mist hail sunset sunrise shadow sky
    """,
    "places": """
        house castle tower bridge temple palace cabin cottage barn farm school library museum
        market bakery hospital station airport harbor tunnel road street park zoo city village
        town tent igloo hut attic garage cellar kitchen bedroom porch patio balcony chimney
        fountain statue arena stadium theater circus
    """,
    "music_and_play": """
        piano guitar violin cello drum flute trumpet trombone tuba harp banjo ukulele clarinet
        organ bugle kazoo cymbal ball kite yoyo puzzle doll teddy marble domino dice chess
        frisbee balloon whistle trophy medal racket hockey soccer tennis golf bowling puck net
        goal swing slide seesaw rattle
    """,
    "materials_and_colors": """
        gold silver copper iron steel bronze plastic wood wool cotton silk leather velvet denim
        rubber red blue green yellow purple pink brown black white gray crimson scarlet indigo
        amber ivory teal navy maroon cyan magenta beige lilac
    """,
    "people": """
        baker chef farmer doctor nurse pilot sailor captain teacher artist author singer dancer
        clown judge miner painter plumber butcher barber tailor poet actor ranger cowboy king
        queen prince princess
    """,
    "things": """
        circle square triangle diamond heart arrow cube sphere pyramid crystal jewel pearl ruby
        emerald sapphire gem coin key lock box bag sack crate barrel ribbon gift card ticket flag
        banner feather nest bone shell horn tusk claw paw tail wing beak fin saddle lasso bell
        siren bubble puddle snowman snowball candle match campfire firework confetti pinata
    """,
}

VALID = re.compile(r"^[a-z]{3,8}$")


def main(out_path):
    words = set()
    for category, text in CATEGORIES.items():
        for word in text.split():
            if not VALID.match(word):
                print(f"skipped {word!r} ({category}): not 3-8 lowercase letters", file=sys.stderr)
                continue
            words.add(word)
    with open(out_path, "w") as f:
        f.write("\n".join(sorted(words)) + "\n")
    print(f"{len(words)} words written to {out_path}")


if __name__ == "__main__":
    main(sys.argv[1])
