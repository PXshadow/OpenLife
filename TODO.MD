Welcome to Open Life Reborn - A hard Winter is comming / Hardcore OHOL with constant rebirthing world

To play, just use any normal OHOL client and put in the custom server IP! No new Client needed!

Summary: 
- the server is written from scratch in Haxe and is in early BETA, so expect some bugs / feel welcome to report them
- there is winter as main new challange and where there is winter there is spring to grow back nature!
- Wolfs and other deadly animals are double strong in winter!
- Temperature can and will kill you!
- Jump drink some water or jump in a river to cool down!
- Use fire or cloths to warm up or escape into a desert!

NEW Commands:
- say "COINS?" --> guess what it does! But no it wont do it in silence! 
- say "SEASON?" OR "?SEASON" displays current season
- say "?AGE" to get your age
- say "?ALLY" to disokay ally, ally count and closest ally
- say "?SCORE" to get current account score / prestige (MSCORE + FSCORE) / 2
- say "?MSCORE" to get current account male score / prestige
- say "?FSCORE" to get current account female score / prestige
- say ?R or ?REPUTATION or REPUTATION? to get current combat reputation

AI:
- Currently the Ai does not use containers, so use Baskets and Tables to save stuff you dont want the Ai to touch
- Currently Ai wont open doors so use them for now to lock out AI
- AI NPCs can hunt, nurse, eat, feed and craft clothing quite well. It goes for YUM and cravings and feeds you your YUM and cravings.
- The AI can bake and cook some basic stuff 
- The Ai can do some basic farming
- AI can do pottery and basic smithing
- The Ai can defend against attackers and will attack not allied onces if you attack them
- By saying "STOP" or "DROP You can order the AI to stop or drop what it helds
- By saying "MAKE xxx"  you can instruct the ai to make stuff with xxx is the item name
- By saying "NAME?" AI says it name
- By saying "PROF?" AI says its current profession

You can assign jobs to AI with say JOB! or JOB!! to assign jobs to all close AI. Assigned jobs have very high priority (executed after MAKE commands). 
Jobs are: 
'SOILMAKER', 'ROWMAKER', 'BASICFARMER', 'ADVANCEDFARMER', 'SHEPHERD', 'BAKER', 'POTTER', 'FIREKEEPER', 'TAILOR', 'FIREFOODMAKER', 'LUMBERJACK','WATERBRINGER', 'FOODSERVER', 'GRAVEKEEPER', 'HUNTER', 'SMITH'

FARMER is shortcut for BASICFARMER
As example FARMER! (closest AI) or FARMER!! (all near AI)

Commands for followers or close relatives:
- By saying "COME" the Ai will follow you for some time or untill you say "STOP"
- By saying "HOME" you can set the closest oven as their new Home
- By starting the command with "ALL" the commands will reach all close AI

 The AI might or might not manage to craft the item. In theory it knows the hole crafting tree, but there might be bugs or needed stuff might not be close enough. Some stuff like making bowls if there are too many might be forbidden for AI.

Combat:
- combat is allowed but might give some prestige mali like attacking children, relatives or allies...
- combat is more skill based / the more close you hit where the playere is the more damage you do
- combat is affected by many other paramters like how healthy you are (max food pipes), if you are in the right biome, if you have clothing...

Map with rivers mountains and oceans:
- the server uses a limited hand made map which can be changed using a picture
- rivers oceans and mountains that block movement (you can build brides with floors)
- no locked iron or wells
- all natural stuff can re-spawn (TODO except natural springs and tary spots)
--> So having a lucky spot of one thing keeps it lucky also in the future
- all can decay and be reborn! (TODO except wells and oil stuff)
- Ancient stuff will decay very very slow

Boats:
- cars are now boats (TODO need new boat graphics for cars)

Reconnection:
- A normal restart / update takes like 30 seconds
- All playerdata is saved, so after server restart you can reconnect.
- The Ai takes over your payer untill you reconnect

Fathers:
- you can now have a father

Curses based on graves:
- curse system is now based on graves
- you incarnate more likely to your grave with a grave stone
- you incarnate less likely to your grave without a grave stone
- you are cursed if going close to your grave without grave stone
- You can curse a grave additionaly by adding a sharp stone
- Cursed players have lower speed, make less damage, receive more damage
- Cursed players have a black writing

Different Classes:
- there are three different classes: Serf / Commoner / Noble depending on your prestige score
- you are more likely born to the same class
- Each class has damage boni with different weapons / Sword for noble

Prestige System:
- yum chain is now your prestiege (score) / health and influcences movement speed, hitpoints and aging
--> so yes in theory you can life longer then 60

Hitpoints / Exhaustion:
- Your max food storage pipes (bootom left) is also now your hitpoints / stamina
- If you are injured or Exhausted you loose Max Food Store pipes. Exhaustion will recover faster than hits.

Reworked Yum and Craving System:
- new craving system that shows you what has full YUM boni
- The more you eat from one food the less food vlaue it gives (minimum halve)
- If you eat other food (especially YUM) in between a eaten food may become YUM again
- Cravings and YUM is inherited from grandparents (or parents if there are none)
- More details to YUM / Craving see below

New in game Currency:
- there is a new in game currency see more about below (Coins)
--> I give you XXCVIIII ==> gives 129 coins C = 100 / X = 10 / V = 5 / I = 1 (order does not matter)

Quality of Life Changes:
- you can pick berriers directly with a bowl
- you can pick up dry beans with a bowl
- soaked dry beans can be cooked with a hot oven
- kids can carry one item (with max one use) and pick berries
- you can do smithing on a table
- escape time for rubber horse cart 7--> 20 sec / and moves 4 --> 2
- escape time for horse cart 7--> 15 sec / and moves 4 --> 2
- you can use a Stone Hoe on a Clay Plate / Bowl to make Clay out of it
- you can use a Stone Hoe on a Basket to get a Reed Bundle
- you can use a Stone Hoe to dig in bones
- you can use flint to undo stone hoes and hatches
- you can unsharp a sharp stone on Big Hard Rock
- you can eat while on a horse (dont know yet if this is a good idea...)
- wearing two shoes give extra speed
- you can use a Steel Mining Pick to get coal out of a Bear Cave (Hungry Work)
- you can use a Drop Spindle on a Small Ball of Yarn to get a Rope 
- Sheep take longer to grow but also take longer to starve. So you have more time to feed!
- Wolfs, Bison and Bears give meat

Better fire Handling:
- you can use Straw on Ashes to start a fire again
- you can use Smithing Hammer plus Stakes to make a Ember Leaf for making a fire
- you can use Ember Leaf on Straw to start a fire
- you can burn rag cloths in fire to make fast fire
- you can burn a Basket of Coals on a fire to make a Large Slow Fire
- you can burn straw in a fire to make a flash fire

Water Handling:
- Better wells always refill faster then primitive wells
- better wells need less hungry work per water gained / using lakes is worst

And the best on the end Better Snake Stuff!
- Sandal: increased decay time from 5h to 30 days
- Snake Skin Boot: Increase decay time from 5h to 10 days
- Double insulation from Snake Skin Boot

Not yet implemented:
- Ai territory
- reading / writing
- twinning
- ally gates / ally spring doors
- in door temperature calculation
- advanced stuff like rails, pipes, plows, cars
- see more in TODO below

How to setup the server you can find in the bottom!
Thx @Colin there is now also a more detailed manual:
https://github.com/connorhsm/OpenLife/blob/master/SERVER.md
And a script to setup the server:
https://github.com/connorhsm/OpenLife/blob/master/build_server.sh

SERVER - FULL CHANGES TO VANILLA
======
SCORE / PRESTIGE:
- YUM-Multiplier is now your score / prestige / health
- your prestige / health influences your aging and max food pipes and therefore speed
- your prestige is also your health displayed where the YUM-Multiplier was.
- on death your new score  is 9/10 of the old score plus 1/10 of this life score
- you have a seperate score for male and female lives. Your total score is one halve each.
- you get extra prestige if you eat / feed YUM or if a follower / (grand) child or sibling gets some prestige
- you get prestige if your mother gets some prestige
- as mother you get prestige for your offspring up to 10 generations even after your death
- you get only prestige for your ancestors if your body from that life is burried and the grave still exists
- you loose score for a cursed grave or if no one burried your bones before they decay

MAP:
- generate limited map using image color as index for biome id
- made map "round", so you can walk over both x,y borders and come out on the other side. 
- has rivers and oceans  
- you can build bridges over rivers and oceans with boards or stone
- iron can be used by default 
- no dry spring and dry tarry spot
- no gradients
- no expert waystones

Adam / Eve:
Deactivated: - adam / Eve do 50% less damage
Deactivated: - adam / Eve receive 50% less damage (also from animals)

Family Temperature:
Your Skin Color affects the temperature you like:
- Black: +0.1
- Brown: +0.05
- White: -0.05
- Ginger: -0.1

Family / Loved / Hated Biomes:
- loved Biomes: Ginger --> Snow / White --> Grey / Brown --> Jungle / Black --> Desert 
- loved food: Ginger --> Wild Carrot / White --> Wild Garlic / Brown --> Banana / Black --> Cactus Fruit 
- loved food2: Ginger --> Cooked Fish / White --> Bowl of Sauerkraut / Brown --> Mango Slices / Black --> Cooked Rabbit 
- loved food count is restored more quickly, so it is more often YUM or craved
- harvesting from loved plants can give extra harvested food if lucky 
- children are now more likely to be born closer to which is best for closest biome
--> With this one Eve can create all colors if people migrate to different biomes...
- parents influence which biomes are loved 
- bad Temperature is affected now by how much you love the biome (see under temperature)
- you gain extra exhaustion in not loved biomes and loose extra exhaustion in loved biomes
- you get  less / more damage in loved / not loved biomes

Classes:
- You are born into a class depending on your prestige from past lives
- there are three different classes: Sef / Commoner / Noble
- you are more likely born to parents from the same class
- nobles make 20% more damage with a sword
- nobles receive 20% less damage if using a sword
- AI: The lower the class (Serf/Commoner/Noble) of the AI the slower it is and the less food it needs

Leadership
- You overtake your leader if you are more 20% more powerful (100% if leader is not your family)!
- Your power is your prestige plus coins and if you are grown up enough (max food pipes)
- leader has now 15 seconds time to exile you before you follow him
- the more your close (5 Tiles) ally stregth is vs enemy streght the more damage you do
- stregth of players with weapons are counted twice

Combat / Attacking
- you can attack if you or the target are angry / terrified enough
- you get angry / terrified 5 seconds after attacking / being attacked
- Double Anger reduction in Passable River / Double in Desert
- you can attack an ally who is holding a weapon without any mali so you can duell
- In case you hit your ally you make halve damage
- In case you attack an ally without any weapon your Top Leader will auto exile you
- The more / less strenght (max food pipes) you have the more / less damage you do 
- clothing insulation reduces damage up to 66%
- Floor insulation is added to clothing insulation up to a damage protection of 75% with full cloths
- you have reduced speed if close to enemy with weapon
- already wounded players receive less damage
- different weapons have different damage and cooldown
- if you make a wound or kill some one the bloody weapon cooldown is much longer
- You can only shoot with bow if not too close to target
- you do halve damage to an ally
- you get less / more damage in loved / not loved biomes
- you receive less damage if holding a knife, sword or riding a horse without a cart 
- males do X1.2 damage
- Reduced weapon pickup age but remember you do less damage if not grown up (kids have max food pipes)

Attacking prestige cost:
- you will loose prestige for attacking a close relative without weapon
- you will loose prestige for attacking a women without weapon
- you will loose even more prestige for attacking an ally without weapon
- you will loose even more prestige for attacking a child without weapon
- if you attack someone who is not holding no weapon or has no a bad reputation you loose reputation
- If you have a bad reputation others may attack you without them loosing prestige
- Attacking some one who has a bad reputation increases your reputation
- Nobles / Commoners loose only halve as much reputation for attacking lower born
- per year you regain some lost reputation
- The lower your reputation is the more you need to pay to hire people
- all the presige and reputation damage is propotional to done damage

Graves / Curses
- instead player curses, curses work now through graves without a grave stone
- since graves are more important you can use Stone Hoe to make graves
- you incarnate more likely to your grave with a grave stone
- you incarnate less likely close to your grave without grave stone
- you are cursed (lower speed / make halve damage / receive more damage) if going close to your grave without grave stone
- you cannot interact with your own grave (except on low population server)
- for each Sharp Stone a grave needs much longer to decay (+8 hours) / This can be used to let cursed graves exist much longer
- If allowed in the settings you can teleport to your own grave with a grave stone by saying "!TG" or "!TGRAVE"
- Teleporting to your Grave costs some points

Heat / Temperature:
- bad temperature reduces speed (you get get an heat / cold animation warning you)
- having very bad temperature causes damage (especially on the far temperature edges)
- bad temperature generates exhaustion and reduces exhaustion reduction
- different color people love different temperatures (Ginger 0.4 White 0.45 Brown 0.55 Black 0.6)
- clothing can protect against cold and heat.
- The better clothing protects against cold the less it protects against heat
- you heal only if not too cold or hot
- you have a better chance of having a kid with right temperature
- heat objects like fire have halve temperature impact if the impact is negative

- You need to stay halve the time in good temperature then you have been in bad temperature
- River / Water affect twice as much and ignores clothing protection
- Clothing Insulation reduces bad temperature impact per second by up to 10X 
(Already 2X with a simple straw hat)
- Clothing Insulation does not reduce temperature impact that is positive for you!
- You have a natural heat resistance which is reduced with wearing cloths with a higher then 25% insulation
- Wearing cloths with less then 75% insulation increases your heat resistance
- Wearing cloths warms you up to at max plus 0.2 heat (this can affect you also if net negative)
- Wearing cloths with less then 50% insulation reduces heat by at max 0.2 (only if net positive)

With full insulation you can run 20X time outside then you need time spending on warming up / cooling down.
With river cooling faster and natural heat resistance you could spent 10X time in heat then to cool down.
(2X River, 2X Good temperature, 2.5% Natural heat protection)

Loved Temperature:
- Temperature is affected now by how much you / your parents love the biome
==> For example a Brown child has temperature boni of 0.1 in Jungle
==> a White child born to brown mother has temperature boni of 0.05 in Jungle and 0.1 in Mountain
==> a White child born to brown mother and father has temperature boni of 0.1 in Jungle and 0.1 in Mountain
==> a White child born to brown mother and ginger father has temperature boni of 0.05 in Jungle, 0.1 in Mountain and 0.05 in Snow (a true multiculti kid)

Exhaustion:
- half of hungry work becomes now exhaustion
- exhaustion reduces food store max untill recovered
- recovering exhaustion increases food need 
- you only recover exhaustion if not wounded, not being in bad temperature and not starving
- starving causes extra exhaustion
- males recover exhaustion X1.2
- bad temperature generates exhaustion and reduces exhaustion reduction
- good temperature increases exhaustion reduction
- // you get extra exhaustion in not loved biome (more in swamp or river if not on floor or bridge)
- you recover exhaustion in your loved biome (like brown in jungle) 
- changing your movement while moving generates exhaustion
- trying to hit someone causes exhaustion
- getting damage increases exhaustion

World:
- paradise is back! Wild Berries and Bana come finaly back with time 
- objects respawn near original objects with a small chance
- objects except ancient, well, Iron Mine, Oil Pumps decay with a small chance
- decay chance is 10X reduced if on floor and 10X increeased if food
- dacay chance is reduced for permanent stuff
- dacay chance is increased for stuff in water
- Ovens Forges and some other objects have custom decay transitions (to Adobe Rubble)
- TODO iron and steel piles currently dont decay

Coins / Currency:
- you get now coins for eating YUM, more for eating cravings
- you can give coins by saying: I GIVE YOU XX with XX = I=1 V=5 X=10 L=50 C=100 D=500 M=1000. The order does not matter (not Roman)
- You take 8 out of 10 coins if you manage to wound some one
- you can hire AIs with coins with say: "I HIRE YOU"
- the hire costs depends on the class of the one you want to hire and your combat reputation
- If you are a close relative hiring costs are lower
- If you arnt allied and dont have the same color the hire cost might be higher
- If you have bad reputation hiring costs more but restores some of your lost reputation
- leaders get 1/5 prestiege / coins their followers get
- coins decay 1% per year if >= 100 coins
- for each deacyed coins you get one prestige (up to 5 prestige per year)
- if you kill some one you get 80% of his coins
- on death you will inherit your coins to same family or ally according to their past actions
- if still some coins are left your kids will inherit your coins
- you can store up to 100 coins in a wooden chest / just open and close

Seasons / Winter is comming:
- there are winter / summer / spring / autoumn. 
- there are hard / super hard variants of it longer / colder / hotter then normal
- seasons influence temperature
- in winter snow biome spreads and melts in spring or summer
- winter decays wild plants and spring lets it grow back
- plants grow only back if either one was left or one was hidden in he winter or if you place one...
- if you eat all, still they can come back since nearly all objects can respawn, but this can take lot of time.
- there is now a second layer of objects where stuff can be hidden like onions in the winter

Healing / Hits
- max food store equals hitpoints
- you Heal only if not starving, too cold or hot, or wounded
- healing is faster the better the temperature 

Wounds
- wounds heal with time accoring to player health
- wounds do damage / exhaustion with time 
- with smal wounds you can still do stuff, but you will still get damage and less speed

Objects: (Needs to be updated)
- cooked Rabbit Food Value: 10 --> 20
- turkey Slice on Plate Food Value: 17 --> 20
- omelette Food Value: 12 --> 15
- Bowl of Cooked Beans 12 --> 20
- cooked Goose: useChance = 0.3;
- compost has less uses to make soil more rare, since water is more plenty: Soil (Wheat, Berry, Dung) + water ==> 4 Soil  

Ownership:
- normal Gates are now inherited by a follower with the highest score
- if there was no follower the one the former owner was following inherits it
- if none was there to inherit the gate the first who finds the gate will own the it

Property:
- Coins can be stored in chests
- Wooden Doors / Chests can be locked with lock and key
- There is only a one in 10000 chance that an key can open another lock
- You can use the same Key for All locks, just place it in mutliple open locks
- Owners of a locked item can open it without a key also in their next lifes
- You can lockpick with a 5% success and 10% fail chance
- Lockpicking costs 1 coin and 3 exhaustion per try
- Lockpicking cost 50% less exhaustion for female
- Lockpicking fails only 8% for female

Fortification:
- Removeing (Ancient) Stone Wall is now hungry work
- (Ancient) Stone Walls cam be fortified with Cut Stone 
- Removing Fortification is Extreme Hungry Work
- Fortification cost 10 Coins for (Ancient) Stone Wall

Transitions:
- you can harvest Berries directly with a bowl
- tule Stumps grow back after one hour 
- if you kill a Wolf you get meat (mutton) 
- you can cook meat (mutton) on hot coals

Aging:
- starving to death slows down aging below adulthood and increases above  
- higher food use for age below adulthood if not starving to death / they need to grow!

SPEED:
- Speed is changed dpending on your max foood pipes (AGE / DAMAGE / YUM) if not on horse???
- AGE: since low and and older age can limited max food pipes it can also reduce speed
- objects in container / sub container lower speed little bit 
- some objects like iron are heavy now and reduce speed if carried
- horses and cars and carts are slower in bad biomes
- road reduces speed mali of bad biome and heavy objects
- having both shoes give now X1.1 Speed
- bad temperature reduces speed extreme temperature reduces double???

Food - YUM:
- YUM chain is used as prestige and health
- eating YUM / Cravings increases Health / Prestige, while eating MEH reduces it
(YUM +1 CARAVING: +2 (or more) MEH: -0.5 SUPERMEH: -2)
- a Food is SUPERMEH if it has only halve of the food value. Eating SUPERMEH reduces health fast
- Eating Supermeh provides +1 Food, but reduces Health  / you trade short term survival for shorter life span.
- if you hold any food the closest most YUM food / craving for you is displayed (TODO consider containers)
- only eatable food is displayed. So if you are not hungry no food is displayed 
- you can only eat if you have space for at least 1/3 the food value, not counting YUM or food reduction 
- YUM gives 3 food boni first eaten. 
- the first 3 times a food is eaten the food is YUM
- food value is reduced (1 for YUM and 0.2 for MEH) for each eaten
- food has at least half the food value
    For example Berry default food 4: 7 YUM / 6 YUM / 5 YUM / 4 MEH / 3.8 MEH / 3.6 MEH ... 2.2 MEH / 2 SUPERMEH / 2 SUPERMEH ...
- if you eat YUM 1 pip (MEH 0.2) is restored with a 0.8% chance from a random eaten food. 
- if the random chosen food is the same food, no food value is restored
    For eample eats a Yum berry / then eats a YUM onion / 80%  x 50% = 40% chance that eating YUM onion restores 1 pip of berry food value 
    For eample eats a Yum berry / then eats a YUM onion / Then eats a YUM carrot / 80%  x 66%% = 52% chance that eating YUM carrot restores 1 pip of berry or onion food value   

Generational cravings:
- a girl inherits the cravings and food eaten count from his gradpa / father in case there is none
- a boy inherits the cravings and food eaten count from his gradma / mother in case there is none
- with each generation some of the eaten food count is restored
- there is a limit on the inherited eaten food count, so that each food is at least once yum for the child

Food - Health - Hits
- YUM chain / Health influences now aging (0.5X to 2X) and speed (1.2X to 0.5X) / With this Berry munching is even worse
- having less hitpoints / beeing more exhausted reduces speed  
- the more healthy you are the more resistant you are against moskito bites and yellow fever

Food - Cravings:
- cravings display now a if possible close food with a full YUM Boni
- there is a Chance that a new complete never eaten not close random food is chosen which is tech wise close to the last craved one
- cravings give plus one extra food boni and YUM chain / Health which influences speed and aging

Animals:
- biome animals wont most of the time attack you if you are in your biome... unless you make them angry...
- Knife and Sword does damage to attacking animals and might kill them
- you can use Knife and Sword to attack animals (please test)
- animals can multiply and die
- animals can leave their biome, but prefare to stay in own biome
- Sheep and other domestic animal try stay close to Green biome
- In Winter Animals (Or Summer in case of Snakes) will attack close people
- Attecked or injured animals will attack close people
- stuff like trees and rabit holes wont block animal movement // TODO also for player / needs client support
- horses are the ultimative devourers you can eat anything from horse!
- animals have more hit points so a shot / hit may or may not kill them

Yellow fever:
- moskito do only echaustion damage but can give yellow fever. 
- The more you love Jungle biome the less moskito damage you get
- the more moskito bites and yellow fever you got the more you are resistant
- the more healthy you are the more resistant you are against moskito bites and yellow fever
- held kids get 5X less damage from yellow fever
- eating lovely mushrooms give some protection against fever and reduces fever time
- you can feed lovely mushrooms to other if they have fever

TODO Features: 
more up in the list means more likely to be implemented first but no guarantee!
- animals grouping / animal bosses / wolf ilness
- illnesses (can spread dependign on health, more food use no eating)
- better property / fortification for other clay walls / oven / kiln / forge / graves 
- better fences / allow to climb over fences, but get wounded / make destroyable if not protected by close gate)
- special stuff like hercules / snake master / super fast ager 
- job boni
- horses that follow owner / horse ownership
- better dogs
- princess / princess

TODO Missing:
more up in the list means more likely to be implemented first but no guarantee!


List off stuff that Ai drops in the wild and might need a fix:
many single bowls
many bowl stacks in wild, seen an ai fuckin come down to stack one
SOOO many knifes
unforged crucible
good few tongs, not as much as knifes
hatchets
adze
drop spindle
couple of skewered rabits
few axes
bows
hoes
mutton
firewood (singluar placed around)
snare
firebow drill
cut rabbit fur
rabbit fur
snake skin
bowl with corn kernles
wet bowl in tongs
many iron ore
iron ore in tongs
pick
shears
bowls of flour
i found a couple or more than a couple of all of em
*also a wooden disk
and uh charcoal

High priority:
- AI not close kiln if clay stuff is to make / check in craftitem for transition
- AI SWORD: get angry if one has one and is not noble

Combat:
- AI use Knife / Sword for combat
- AI dont engage in combat if cursed and opponent is not cursed
- AI dont run back if fleeing / mark target tile hostile

Reputation:
- AI dont dig in bones from not allied people with bad reputation

Commands:
- AI add new command STAY CLOSE
- AI hire cost command
- AI add new command BRING
- AI add new command craft --> constanly tries to craft untill 5 are reached

Follow:
- AI auto change leader if in new home
- AI auto set home from direct leader or change leader if want stay in same home

Crafting:
- AI cache couted objects and objects for crafting

Healing / Feeding:
- AI feeding water to heated players
- AI healing other players

Cointainer:
- AI eating from containers (needed also for player YUM display)
- AI storing pies in containers 
- AI using of backpack (baskets) / for example kindling, potato, pies to eat / water

Generel:
- FIX AI loot grave while fresh
- FIX wrong death messages / stuck on death
- FIX teleporting Ai especially if you drop BB
- FIX AI droping stuff outside of the village Fixed???
- FIX exile only from leader if you are no noble and have no bad reputation 
- FIX Ai dont make tables
- FIX set home if touching oven
- FIX trekkieA: issues with objects disappearing from backpack, dont swap items quickly or you might lose it
limestone was weird, if you use hetuw. you can crtl+q to swap items in bp. if you have more than one item swapping them can destroy objects
- FIX trekkieA: hired ai wont change homes, even when luring into range. just tend to walk back to where i found them 
- FIX trekkieA: visual bug when filling bowl of berries on bush, only see 5 berries instead of 6 - berry thing goes away after moving a tile
- FIX trekkieA: when either sheep or squito (not sure) stand on top of hand cart. it gets stuck in forever motion. constant squeak
- FIX player objects like wounds and cloths / stuff in backpack??? dont seem to be updated if data version changes

- switch leader position if 20% more leader power
- dont let bones decay on floor
- exile if you wound an ally and close by
- attack exiled
- dont allow hiring if seen exiled
- !ally --> count allyies
- fix extra carrots
- add !topscore command
- add !topcoins command
- add !prestige command --> lists gained prestige
- let buffalos heal with one arroy
- get angry if stealing stuff
- get angry if dropping snakes in none desert
- migrate if no food
- dont migrate too if exiled / migrade away if not strong enough
- ally up in new town
- Ai dont take last water from goose
- let sons follow with 80% their father if aged
- let daughters follow with 20% their father if aged
- dont allow build permanent stuff in deep water / mountain if not on floor except floor or floor stakes 

- MASKS: Extra damage

- BIOME let desert spread
- BIOME let water spread
- BIOME let snow spread also in Jungle / Desert
- WINTER snow banks / melting if snow

- FOOD nerving raw foods like carrots
- FOOD make ginger love food carrot pie instead of carrot after generation 3
- FOOD auto balance food pipes globally if eaten too much

- WRITING / READING
- SAVE secret for player
- POPERTY Ally Gates
- POPERTY Done for Stone Wall / other foritfication missing + stone + hungry work

- Animal hide rabbit / sanakes in snow
- Animal Let Bears go in cave before winter 
- Animal Let wolfs / bears eat sheeps / cows
- Animal Let pigs eat food
- Animal let pigs grow back faster
- Animal let pigs hide in swamp
- Animal let wolfs hide behind trees

- AI: FIX pathing bug
- FIX? /leader Client crash (if leader or anybody? is out of range??) 
==> For now you can use insteads !LEADER (!L) or !DL for direct leader (first you follow)
==> as workaround all player data is send always ==> does still not work


- FIX clearing up old lineage data --> needs a fix if player is still alive or if anythign is updated later on from the linage like family Name

Middle priority:
- Give Escape Snake bite Boni for Snake Boots / Sandlas
- Give sandlas high heat protection / implement custom cloth heat protection
- ALLow building bridges only with 4 stone walls close, except stone bridge
- BIOME better water drift meechanics
- TEMPERATURE indoor temperature calculation 
- Account Authentication (Maybe a system based on IP limits plus higher prio for players with higher score)
- AI build oven / klin / forge on a good location
- AI FIX bug with transitions that need two input like 2X thread for rope. thread --> bowl --> thread 
- AI if waiting for time transition check if the next step can be prepared --> fire for now is manually optimized in craftItem
- Heating for building / Biome temperature changes

- AI using containers 
- AI help building roads / repair roads
- AI better pathing towards home

- TIME time transitions for objects in container that are in containers (test if implemented)
- TIME decay stuff in containers??? --> baskets decays now
- TIME decay for wells
- TIME decay colored walls
- TIME allow foloors to decay if walled by anicent walls? or generall alow floor decay?
- ??? TIME decay all clothing

- FIX PICKUP / DROP CHILD fix display bug. maybe set origion or action right?
- FIX Needle and Thread --> Bone Needle (How does this on earth work???)
- FIX light wounds are not light wounds anymore after reconnect / server restart?

- SAVE season / save animal loved biome spot 
- SAVE save more stats for player, like best play, different eve runs, number of played, last loged in, amount of time played

- DISPLAY give hints for crafting

- BIOME give biome experience for staying in biome or eating biome food
- ??? BIOME biome hide biome specific stuff for not specialists
- ??? BIOME change biome speed for specialists (only if in attack mode or carrying heavy stuf)
- ??? BIOME add exhaustion for doing stuff in bad biome
- ??? BIOME make moskitos for all bad in swamp

- ALLY test: change ally color if leader changes 
- ALLY give some time after beeing exiled

- MAP use bigger map (Europe?) / create tool to auto create map

- EXHAUSTION carrying heavy stuff
- EXHAUSTION for working in bad biome
- EXHAUSTION for more tools / tools always better then primitive tools
- EXHAUSTION better indoor exhaustion reduction

- COMBAT block movement if other player is hostile 

- CRAFTING crafting on table

- RAILS implement rails

- ANIMAL animal movement on top of items --> Fix bug so that item apear again if animal interacted with 
- ANIMAl eating sheep / wolf meat transitions 
- ANIMAl snakes hide in winter and come back in sommer
- ANIMAL leaders
- ANIMAL advanced player like animals

- SCORE give prestige boni for clothing (currently you get for crowns)

- AI guards 

- MEASSAGE add bad biome say to self meassage (not needed? / what to do with bad biomes)

- CONNECTION twinning

- STATS implement basic server stats (done for food, objects linages)

- POINTER bell tower ringing
- POINTER maps / maps on waystone
- POINTER home marker --> Seems to work

- ADVANED oil stuff, pipes, sprinkler, plow, roadmaker
- TOOLS use list of learned tool and set held_learned only true if tool is learned 

- GATES visitor at gate

Low priority stuff:
- AI pickup stuff after droped for eating???
- ??? YUM use yum category // for example for berry / berry bowl
- PLANES
- RADIO
- LANGUAGE implement different languages
- tatoo
- photo

PX TODO
====
1. Transition history system (used by both bot and server)
2. Implement another path finding algorithm
3. Algorithm to score tech tree climb, time dependent, and based on future potential for survival as well
4. Solo Eve Bot run, scored after 1 life and world is static and reset each time
5. Task Queue system with Auto.hx class
6. Algorithm to identify what is needed next for overseer

IS this used? https://www.geeksforgeeks.org/a-search-algorithm/

How to change Server config:
- there is a ServerSettings.txt file in SaveFiles folder. Settings are read at start and every 10 sec.
- server uses cached ObjectData to start fast
--> if you want to clear the cache you need to use another .\OneLifeData7\dataVersionNumber 
--> or delete the cache file file: .\SaveFiles\saveObjectData.bin
- you can use another map 
--> just override the current used one, or change the map file name in the Server ServerSettings.txt
- every 10 sec the server state is saved in the SaveFiles directory 
- every 8 hours a backup is made which is cycled through (up to 90 backups) / this can also be changed in ServerSettings.txt
--> the backups are in the folders: SaveFiles/1/ SaveFiles/2/.... SaveFiles/9/ 

For debugging:
if AllowDebugCommmands = true in ServerSettings.txt:
    can create objects with saying "!create ID" / "!create object" "!create object!" with ! indicating that object ends with "object" or test wounds with using "!hit" or "!heal"
    --> for example: "!create ore!" creates iron ore. "!create iron" creates the first object with iron in the name.

Server setup:
====
Best is use Unbuntu, got tons of problems with (default) Debian...

From windows putty can be used to connect to server:
putty.exe -ssh root@x.x.x.x -pw password

If you dont know which Linux distro the server has:
Get current Linux Distroinfo: 
ls -la ~/ | more

Install haxe (best use Unbuntu) Haxe Version must be 4.2 at least (Debian has old version by default so setting up haxe is tricky on Debian (beginning 2022)):
https://haxe.org/download/linux/

Get code from Github:
sudo apt-get install git

git clone https://github.com/PXshadow/OpenLife

Get OHOL data from Jason option 0 (Jason): 
haxe setup_data_server.hxml
// if SetupDataServer does not work try: haxe setup_data.hxml 

Install Hasklink to execute Haxe hl files:
Dont forget to install gcc first to compile hashlink :)
https://github.com/HaxeFoundation/hashlink.git 

Compile OHOL server (server.hxml --> server.hl):
haxe server.hxml

Run server:
hl server.hl

In case of this error:
hl: error while loading shared libraries: libhl.so: cannot open shared object file: No such file or directory

Fix it with this:
https://github.com/HaxeFoundation/hashlink/pull/483/commits/e1f78d94cf304c0ea6d873a7a523de6b945c335e

This might also help to fix libhl.so:
sudo make INSTALL_DIR=/usr install

Install tmux for letting the server run after disconnect:
sudo apt-get install tmux

https://www.howtogeek.com/671422/how-to-use-tmux-on-linux-and-why-its-better-than-screen/

tmux
then start the server in tmux with: hl server.hl
then detach: If you press Ctrl+B, and then D, you will detach the session.
the server should run now still in a tmux session even after you close your connection
Dont forget to remember the session name so that you can attach again!

If you want to shut it down attack with tmux:
tmux attach-session -t sessionname

And then press "ctrl" c to stop the server.

Helpful stuff:
winscp is a nice graphical tool for windows

Emacs for edditing files in terminal: (option -nw)
emacs -nw filename
rm -r SaveFiles // to remove a directory like if you want to delete the SaveFiles folder so that the server creates all new 

Dont forget that there is a ServerSettings.txt file in SaveFiles folder. Settings are read at start and every 10 sec. If there is no folder or settings file it should be automatically created on first server start. 

Update Server:
Pull new code from github (must be in OpenLife dir):
git pull https://github.com/PXshadow/OpenLife

Compile OHOL server:
haxe server.hxml

In case you changed also the data: Get OHOL data from Jason option 0 (Jason):
haxe setup_data_server.hxml

Run server:
hl server.hl

If you want to create the map from scratch delete SaveFiles folder or the stored stuff inside

In case you want to programm:
Setup Development (Windows):
- Install VS Code
- Install Git
- Install Haxe: https://haxe.org/download
- Install Hashlink (Debugging Haxe)
https://stackoverflow.com/questions/56228386/how-to-install-hashlink-on-windows
Download: https://github.com/HaxeFoundation/hashlink/releases
Then go to the System Properties -> Environment variables, edit the path variable then add the location of your hashlink location that you saved.

In VS Code Install (Use the newest version if there is):
Haxe language Support 2.27.0
Hashlink Debugger
Lix v.1.3.0

If Haxe cannot be found restart VS Code after installing haxe
If Hashlink Debugger (HL) cannot be found make sure that Hashlink is installed an PATH is set right. Than restart VS Code.

If this error comes: Error: Error: Cannot process [format]: Library format is not installed:
Run this in VS Code Terminal
haxelib install format

Get OHOL data from Jason option 0 (Jason): haxe setup_data_server.hxml // if SetupDataServer does not work try (run in terminal): haxe setup_data.hxml

Update:
git push -u https://github.com/PXshadow/OpenLife

Download last:
git pull https://github.com/PXshadow/OpenLife

Programming:
How to Push stuff to git:
git push -u origin master
or:
git push -u https://github.com/PXshadow/OpenLife

How to pull stuff from git:
git pull https://github.com/PXshadow/OpenLife

How to connect to the test server:
(Currently the server is written totally new and is in Alpha, so expect many bugs and feel free to report them in the forum or even better on github! There might be an AI running which is currently very basic. It can nurse, eat, feed, craft. It does not know about containers, temperature, bad biome or clothing yet. And crafting might fail. Nevertheless you can say to the Ai to craft something with saying "make xxx" which might or might not work... Later on the plan is that you can hire Ais to become your followers with the currency that is allready in):
ip: 37.120.188.155
port: 8005

In case you want to try stuff out on an original server:
Jason server:
server10.onehouronelife.com
