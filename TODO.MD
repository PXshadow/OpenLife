Project Winter is comming / Hardcore OHOL

How to setup the server you can find in the bottom!

Project Winter is comming summary: 
- the server is written from scratch in Haxe and is in ALPHA, so expect errors / feel welcome to report them
- there is winter as main new challange

Life is generally more hard / deadly:
- you are generally more hunted by animals, yea once in a time human where prey... animals especially wolfs can leave their original biome, so its more deadly, but no prob, if you help the ai to make a bow and some arrows it will help you to hunt most of them down. Wolfs do spawn now in all biomes except desserts and jungle. So yea better hide in the jungle if brown since moskitos are less deadly for brown :) And once you survived a yellow fever, the next one would be hopefully not be that hard...
// TODO implement that fire protects against animals

AI:
- there might be an basic AI for NPCs if activated in the settings
- by giving "make xxx" commands you can instruct the ai to make stuff. with xxx part of an object name or ID. 
TODO: This might or might not work yet!

Combat:
- combat is allowed but might give some prestige mali like attacking children. relatives or allies...
- combat is more skill based / the more close you hit where the playere is the more damage you do

Map with rivers mountains and oceans:
- the server uses a limited hand made map which can be changed using a picture
- rivers oceans and mountains that block movement
- no locked iron or wells
- all natural stuff can re-spawn (TODO exception well, iron and oil stuff)
--> So having a lucky spot of one thing keeps it lucky also in the future
- all But ancient stuff can decay (exception well, iron and oil stuff)

Boats:
- cars are now boats (TODO need new graphics for cars)

Reconnection:
- Playerdata is saved, so after server restart you can reconnect.
- The Ai takes over your payer untill you reconnect

Curses based on graves:
- curse system is now based on graves
- you incarnate more likely to your grave with a grave stone
- you incarnate less likely to your grave without grave stone
- you are cursed (lower speed / halve damage) if going close to your grave without grave stone

Different Classes:
- there are three different classes: Sef / Commoner / Noble depending on your prestige score
- you are more likely born to the same class

Prestige System:
- yum chain is now your prestiege (score) / health and influcences movement speed and aging
--> so yes in theory you can life longer then 60

Reworked Yum and Craving System:
- new craving system that shows you what has full YUM boni
- Cravings and YUM is inherited from grandparents
- More changes to YUM / Craving see below

New in game Currency:
- there is a new in game currency see more about below (Coins)
--> I give you XXCIIII ==> gives 124 coins

Quality of Life Changes:
- quality of Life: You can pick berriers directly with a bowl
- you can eat while on a horse (dont know yet if this is a good idea...)
- Shoes give extra speed
- Wells fill up at least 3 times faster. Better wells always refill faster then primitive wells

Not yet implemented:
- advaned AI
- boni for different biome specialists
- advaned stuff like rails, pipes
- reading / writing
- twinning
- ally gates / fences
- in door temperature
- general lot more testing of stuff
- see more in TODO below

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
// TODO implement prestige for your ancestors for father / siblings

MAP:
- generate limited map using image color as index for biome id
- made map "round", so you can walk over both x,y borders and come out on the other side. 
- has rivers and oceans  
- you can build bridges over rivers and oceans with boards or stone (TODO let them decay)
- iron can be used by default 
- no dry spring and dry tarry spot
- no gradients
- no expert waystones

Adam / Eve:
// - adam / Eve do 50% less damage
// - adam / Eve receive 50% less damage (also from animals)

Family / Loved / Hated Biomes:
- loved Biomes: Ginger --> Snow / White --> Grey / Brown --> Jungle / Black --> Desert 
- loved food: Ginger --> Wild Carrot / White --> Wild Garlic / Brown --> Banana / Black --> Cactus Fruit 
- loved food count is restored more quickly, so it is more often YUM or craved
- harvsting from loved plants give in average double food items (TODO White and Ginger)
- children are now more likely to be born closer to special biome color
--> With this one Eve can create all colors if people migrate...
- parents influence which biomes are loved 
- bad Temperature is affected now by how much you / your parents (50%) love the biome
- you gain extra exhaustion in not loved biomes and loose extra exhaustion in loved biomes
- you get  less / more damage in loved / not loved biomes
- TODO differen exhaustion for work in bad / loved biome

Classes:
- there are three different classes: Sef / Commoner / Noble
- you are more likely born to the same class
- nobles make 20% more damage with a sword
- nobles receive 20% less damage if using a sword

Ally
- leader has now 15 seconds time to exile you before you follow him
- allow only pickup if not too close to not allies with more total strength (5 Tiles)
- the more your close (5 Tiles) ally stregth is vs enemy streght the more damage you do
- stregth of players with weapons are counted twice

Combat / Attacking
- you can attack anybody if you are angry and if target is terrified or angry
- you get angry / terrified 5 seconds after attacking / being attacked
- The more / leass health pipes you have the more / less damage you do 
- clothing insulation reduces damage up to 66%;
- you have reduced speed if close to enemy with weapon
- already wounded players receive less damage
- different weapons have different damage and cooldown
- if you make a wound or kill some one the bloody weapon cooldown is much longer
- the more your close (5 Tiles) ally stregth is vs enemy streght the more damage you do
- stregth of players with weapons are counted twice
- You can only shoot with bow if not too close to target
- you do halve damage to an ally
- you get less / more damage in loved / not loved biomes
- you receive less damage if holding a knife, sword or riding a horse without a cart 
- males do X1.2 damage
- weapon pickup age is 2/5/10 years for knife / bow / sword but remember you do less damage if not grown up

Attacking prestige lost:
- you will loose prestige for attacking a close relative without weapon
- you will loose prestige for attacking a women without weapon
- you will loose even more prestige for attacking an ally without weapon
- you will loose even more prestige for attacking a child without weapon

Graves / Curses
- instead player curses, curses work now through graves without grave stone
- since graves are more important you can use Stone Hoe to make graves
- you incarnate more likely to your grave with a grave stone
- you incarnate less likely close to your grave without grave stone
- you are cursed (lower speed / halve damage) if going close to your grave without grave stone
- you cannot interact with your own grave 
- for each Sharp Stone a grave needs much longer to decay (+8 hours) / This can be used to let cursed graves exist much longer

Heat / Temperature:
- bad temperature reduces speed extreme temperature reduces double
- different color people love different temperatures (average temperature and resistance to too hot or cold temperature)
- clothing can protect against cold and heat. The better it protects against cold the less it protects against heat
- heal only if not too cold or hot / heal faster the better temperature 
- having very bad temperature causes damage (especially on the far temperature edges)
- bad temperature generates exhaustion and reduces exhaustion reduction
- better chance of having a kid with right temperature
- bad Temperature is affected now by how much you / your parents (50%) love the biome

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

Coins / Currency:
- you get now coins for eating YUM, more for eating cravings
- you can give coins by saying: I GIVE YOU XX with XX = I=1 V=5 X=10 L=50 C=100 D=500 M=1000. The order does not matter (not Roman)
- leaders get 1/5 prestiege / coins their followers get
- coins decay 1% per year if >= 100 coins
- for each deacyed coins you get one prestige (up to 5 prestige per year)
- if you kill some one you get 80% of his coins
- on death you will inherit your coins to same family or ally according to their past actions
- if still some coins are left your kids will inherit your coins

Seasons / Winter is comming:
- first part of Seasons is now implemented
- there are winter / summer / spring / autoumn. And hard / super hard variants of it longer / colder / hotter then normal
- seasons influence temperature
- winter decays wild plants and spring lets it grow back
- plants grow only back if either one was left or one was hidden in he winter or if you place one...
- if you eat all, still they can come back since nearly all objects can respawn, but this will take lot of time...
- tere is now a second layer of objects where stuff can be hidden like onions in the winter

Healing / Hits
- max food store equals hitpoints
- you Heal only if not starving, too cold or hot, or wounded
- healing is faster the better the temperature 

Wounds
- wounds heal with time accoring to player health
- wounds do damage / exhaustion with time 
- with smal wounds you can still do stuff, but you will still get damage and less speed

Objects:
- cooked Rabbit Food Value: 10 --> 18
- turkey Slice on Plate Food Value: 17 --> 24
- omelette Food Value: 12 --> 15
- cooked Goose: useChance = 0.3;
- compost has less uses to make soil more rare, since water is more plenty: Soil (Wheat, Berry, Dung) + water ==> 4 Soil  

Ownership:
- normal Gates are now inherited by a follower with the highest score or if none by the one the dead was following
- if none was there to inherit the gate the first finder will own the gate

Transitions:
- you can harvest Berries directly with a bowl
- tule Stumps grow back after one hour 
- if you kill a Wolf you get meat (mutton) 
- you can cook meat (mutton) on hot coals

Aging:
- starving to death slows down aging below adulthood and increases above  
- higher food use for age below adulthood if not starving to death / they need to grow!

SPEED:
- different speed for different biomes
- limited speed for lower age < 10 and age > 55
- starving to death reduces speed if not in car or on horse
- objects in container / sub container lower speed
- horses and cars and carts are slower in bad biomes
- road reduces speed mali of bad biome and heavy objects
- having both shoes give now X1.1 Speed
- bad temperature reduces speed extreme temperature reduces double

Food - YUM:
- YUM chain is used as prestige and health
- eating YUM / Cravings increases Health / Prestige, while eating MEH reduces it
(YUM +1 CARAVING: +2 (or more) MEH: -0.5 SUPERMEH: -1)
- a Food is SUPERMEH if it has only halve of the food value. Eating SUPERMEH reduces health fast
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
- knife and Sword does damage to attacking animals and might kill them (TODO allow to attack wolfs etc with knife / sword)
- animals can multiply and die
- animals can leave their biome, but prefare to stay in own biome
- stuff like trees and rabit holes wont block animal movement // TODO also for player / needs client support
- horses are the ultimative devourers you can eat anything from horse!
- animals have more hit points so a shot / hit may or may not kill them

Yellow fever:
- moskito do very little damage but can give yellow fever. The more you love Jungle biome the less damage you get
- the more moskito bites and yellow fever you got the more you are resistant
- the more healthy you are the more resistant you are against moskito bites and yellow fever
- held kids get 5X less damage from yellow fever
- eating lovely mushrooms give some protection against fever and reduces fever time
- you can feed lovely mushrooms to other if they have fever

TODO Features: 
more up in the list means more likely to be implemented first but no gurantee!
- fathers
- better fences / allow to climb over fences, but get wounded / make destroyable if not protected by close gate)
- special stuff like hercules / snake master / super fast ager 
- job boni
- horses that follow owner / horse ownership
- angry / bloody weapon if killing sheeps / animals
- animals grouping and hunting in winter
- better dogs
- advanced AI / AI villages
- princess / princess

TODO Missing:
more up in the list means more likely to be implemented first but no gurantee!

High priority:
- AI make commando only to closest
- AI follow human player
- AI FIX just standing around
- AI FIX escape
- AI FIX child: if hungry and close to feeding player dont move 
- AI FIX drop made stuff after ordered in fronts of player
- AI FIX bug with transitions that need two input like 2X thread for rope. thread --> bowl --> thread
- AI FIX bug with wanting to shoot bear in order to get a rope from bow

- AI consider if some one else is already feeding
- AI dropping not needed crafting stuff
- AI if waiting for time transition check if the next step can be prepared
- AI basic clothing
- AI player fight
- AI temperature espace
- AI using containers needed also for player YUM display
- AI go for search if something is needed
- AI test with > 20 AIs
- AI fill baskets with bananas
- AI using of baskets and backpack
- AI consider making / using a pile
- AI Stoneage

- FIX bug with getting stuck sometimes
- FIX bug with reconnect if holding a player // TODO check if it is a problem
- FIX connection crash on connect 
- FIX /leader Client crash if you follow a new leader 

- MAP check why ponds are not in water ==> move to green near water

- MEASSAGE addbad biome self say meassage

- RAND look at random generator for float

- PICKUP / DROP CHILD fix display bug. maybe set origion or action from right?

- BIOME biome boni for different biome specialists like harvesting more banana for brown TODO: White, Black and Ginger 

- ANIMALS fix they wont sometimes display as moving / DONE???

- BIRTH consider deadly animals
- BIRTH fathers

- MAP use bigger map (Europe?) / create tool to auto create map

- SAVE season / save animal loved biome spot
- SAVE save more stats for player, like best play, different eve runs, number of played, last loged in, amount of time played

Middle priority:
- GATES Ally Gates and swing doors

- FIX clearing up old lineage data / make storing lineage data more efficient
- FIX Needle and Thread --> Bone Needle (How does this on earth work???)
- FIX light wounds are not light wounds anymore after reconnect / server restart?

- DISPLAY give hints for crafting

- SAVE for contained objects. Currently only ids from contained obj are saved

- BIOME give biome experience for staying in biome or eating biome food
- BIOME biome hide biome specific stuff for not specialists
- BIOME change biome speed for specialists (only if in attack mode or carrying heavy stuff???)
- BIOME no dropping of stuff on water??? or better let it drift back to land?
- BIOME add exhaustion for doing stuff in bad biome
- ??? BIOME make moskitos for all bad in swamp

- ALLY test: change ally color if leader changes 
- ALLY give some time after beeing exiled

- EXHAUSTION carrying heavy stuff
- EXHAUSTION for working in bad biome
- EXHAUSTION for more tools / tools always better then primitive tools
- EXHAUSTION better indoor exhaustion reduction

- COMBAT block movement if other player is hostile 

- TIME time transitions for objects in container
- TIME decay stuff in containers
- TIME decay clothing
- TIME use item for decay like Broken Skewer  ID: 847
- TIME decay floors / bridges even faster

- CRAFTING crafting on table

- RAILS implement rails

- ANIMAL animal movement on top of items
- ANIMAl eating sheep / meat transitions 
- ANIMAl chasing players
- ANIMAl Winter attacks
- ANIMAL leaders
- ANIMAL advanced player like animals

- SCORE give prestige boni for clothing / crown

- TEMPERATURE indoor temperature calculation 

- WINTER change snow biome / snow banks / melting if snow

- AI Pottery
- AI Advanced crafting
- AI Farming
- AI IRON AGE
- AI guards 

- WRITING / READING

- GATES visitor at gate

- YUM use yum category // for example for berry / berry bowl (maybe not needed?)

- CONNECTION twinning

- STATS implement basic server stats

- POINTER bell tower ringing
- POINTER maps / maps on waystone
- POINTER home marker

- LANGUAGE implement different languages

- ADVANED oil stuff, pipes, sprinkler, plow, roadmaker
- TOOLS use list of learned tool and set held_learned only true if tool is learned 
- RADIO

Low priority stuff:
- MAP calculate distance right when on end of map 
- photo

TODO Testing
- transition testing
- multiplayer testing

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

For Server setup:
Best is use Unbuntu, got tons of problems with (default) Debian...

From windows putty can be used to connect to server:
putty.exe -ssh root@x.x.x.x -pw password

If you dont know which Linux distro the server has:
Get current Linux Distroinfo: 
ls -la ~/ | more

Install haxe (best use Unbuntu) Haxe Version must be 4.2 at least (Debian has old version by default so setting up haxe is tricky):
https://haxe.org/download/linux/

Get code from Github:
sudo apt-get install git

git clone https://github.com/PXshadow/OpenLife

Get OHOL data from Jason option 0 (Jason): 
haxe setup_data.hxml

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
Emacs for edditing files in terminal: (option -nw)
emacs -nw filename
rm -r SaveFiles // to remove a directory like if you want to delete the SaveFiles folder so that the server creates all new 

Dont forget that there is a ServerSettings.txt file in SaveFiles folder. Settings are read at start and every 10 sec. If there is no folder or settings file it should be automatically created on first server start. 

Update Server:
Pull new code from github (must be in OpenLife dir):
git pull https://github.com/PXshadow/OpenLife

Compile OHOL server:
haxe server.hxml

Run server:
hl server.hl

If you want to create the map from scratch delete SaveFiles folder or the stored stuff inside

Programming:

How to Push stuff to git:
git push -u origin master
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
