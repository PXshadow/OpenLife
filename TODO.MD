Project Winter is comming / Hardcore OHOL

How to setup the server you can find in the bottom!

Project Winter is comming summary: 
- The server is written from scratch in Haxe and is in ALPHA, so expect errors / feel welcome to report them
- There is winter as main new challange
- killing is allowed but might give some mali
- combat is more skill based / the more close you target the player the more damage
- there are three different classes: Sef / Commoner / Noble
- you are more likely born to the same class
- cars are now boats (TODO need new graphics for cars)
- curse system is now based on graves
- you incarnate more likely to your grave with a grave stone
- you incarnate less likely to your grave without grave stone
- you are cursed (lower speed / halve damage) if going close to your grave without grave stone
- The server uses a limited hand made map which can be changed using a picture
- Rivers oceans and mountains that block movement
- No locked iron or wells
- All natural stuff can re-spawn (exception well, iron and oil stuff)
--> So having a lucky spot of one thing keeps it lucky also in the future
- All But ancient stuff can decay (exception well, iron and oil stuff)
- New craving system that shows you what has full YUM boni
- Yum chain is now your prestiege (score) / health and influcences movement speed and aging
- There is a new in game currency see more about below (Coins)
--> so yes in theory you can life longer then 60
- Quality of Life: You can pick berriers directly with a bowl, you can eat while on a horse. Shoes give extra speed

--> so I give you XXCIIII ==> gives 124 coins
- Wells fill up at least 3 times faster. Better wells always makes more water then primitive wells

AI:
- by giving "make! xxx" commands you can instruct the ai to make stuff. with xxx part of an object name or ID

Not yet implemented:
- advaned AI
- advaned stuff like rails, pipes is not tested yet, so may or may not work
- ally gates / fences
- saving of players on server restart. 
- see more in TODO below

SERVER - CHANGES TO VANILLA
======
SCORE / PRESTIGE:
- YUM-Multiplier is now your score / prestige / health
- Your prestige / health influences your aging and max food pipes and therefore speed
- Your prestige is also your health displayed where the YUM-Multiplier was.
- ON death your new score  is 9/10 of the old score plus 1/10 of this life score
- You have a seperate score for male and female lives. Your total score is one halve each.
- You get extra prestige if you eat / feed YUM or if a follower / (grand) child or sibling gets some prestige
- You get prestige if your mother gets some prestige
- As mother you get prestige for your offspring up to 10 generations even after your death
- You get only prestige for your ancestors if your body from that life is burried and the grave still exists
- You loose score for a cursed grave or if no one burried your bones before they decay
// TODO implement prestige for your ancestors for father / siblings

MAP:
- Generate limited map using image color as index for biome id
- Made map "round", so you can walk over both x,y borders and come out on the other side. 
- Has rivers and oceans  
- Iron can be used by default 
- No dry spring and dry tarry spot
- no gradients
- no expert waystones

Adam / Eve:
- Adam / Eve use halve food
- Adam / Eve do halve damage
- Adam / Eve receive halve damage (also from animals)

Family:
- Children are now more likely to be born closer to special biome color. With this one Eve can create all colors if people migrate...
- Partens influence which biomes are loved 

Loved / Hated Biomes:
- Bad Temperature is affected now by how much you / your partens (50%) love the biome

Classes:
- there are three different classes: Sef / Commoner / Noble
- you are more likely born to the same class
- Nobles make 20% more damage with Sword

Ally
- Leader has now 15 seconds time to exile you before you follow him
- allow only pickup if not too close to not allies with more total strength (5 Tiles)
- the more your close (5 Tiles) ally stregth is vs enemy streght the more damage you do
- stregth of players with weapons are counted twice

Combat / Attacking
- you can attack anybody if you are angry and if target is terrified or angry
- you get angry / terrified 5 seconds after attacking / being attacked
- clothing insulation reduces damage up to 66%;
- you have reduced speed if close to enemy with weapon
- already wounded players receive less damage
- different weapons have different damage and cooldown
- if you make a wound or kill some one the bloody weapon cooldown is much longer
- the more your close (5 Tiles) ally stregth is vs enemy streght the more damage you do
- stregth of players with weapons are counted twice
- You can only shoot with bow if not too close to target
- wounds do damage / exhaustion with time and can heal with time 
- you do halve damage to ally 
- males do X1.2 damage
- you will loose prestige for attacking a close relative without weapon
- you will loose prestige for attacking a women without weapon
- you will loose even more prestige for attacking an ally without weapon
- you will loose even more prestige for attacking a child without weapon

Graves / Curses
- Instead player curses, curses work now through graves without grave stone
- you incarnate more likely to your grave with a grave stone
- you incarnate less likely close to your grave without grave stone
- you are cursed (lower speed / halve damage) if going close to your grave without grave stone
- you cannot interact with your own grave 
- For each Sharp Stone a grave needs much longer to decay (+8 hours) / This can be used to let cursed graves exist much longer

Heat / Temperature:
- bad temperature reduces speed extreme temperature reduces double
- different color people love different temperatures (average temperature and resistance to too hot or cold temperature)
- clothing can protect against cold and heat. The better it protects against cold the less it protects against heat
- Heal only if not too cold or hot / heal faster the better temperature 
- take damage if too hot or cold especially if far too hot / cold
- bad temperature generates exhaustion and reduces exhaustion reduction
- better chance of having a kid with right temperature
- bad Temperature is affected now by how much you / your parents (50%) love the biome

Exhaustion:
- reduces food store max untill recovered
- recovering exhaustion increases food need 
- males recover exhaustion X1.2
- bad temperature generates exhaustion and reduces exhaustion reduction
- half of hungry work becomes now exhaustion
- exhaustion in not loved biome (more in swamp or river if not on floor or bridge)
- greater recover of exhaustion in loved biome 
- Changing your movement while moving generates exhaustion

World:
- Paradise is back! Wild Berries and Bana come finaly back with time 
- objects respawn near original objects with a small chance
- objects except ancient, well, Iron Mine, Oil Pumps decay with a small chance
- decay chance is 10X reduced if on floor and 10X increeased if food

Coins / Currency:
- You get now coins for eating YUM, more for eating cravings
- You can give coins by saying: I GIVE YOU XX with XX = I=1 V=5 X=10 L=50 C=100 D=500 M=1000. The order does not matter (not Roman)
- Leaders get 1/5 prestiege / coins their followers get
- Coins decay 1% per year if >= 100 coins
- For each deacyed coins you get one prestige
- If you kill some one you get 80% of his coins
- On death you will inherit your coins to same family or ally according to their past actions
- If still some coins are left your kids will inherit your coins

Seasons / Winter is comming:
- First part of Seasons is now implemented
- There are winter / summer / spring / autoumn. And hard / super hard variants of it longer / colder / hotter then normal
- Seasons influence temperature
- Winter decays wild plants and spring lets it grow back
- Plants grow only back if either one was left or one was hidden in he winter or if you place one...
- If you eat all, still they can come back since nearly all objects can respawn, but this will take lot of time...
- There is now a second layer of objects where stuff can be hidden like onions in the winter

Healing / Hits
- max food store equals hitpoints
- player heals with time but healing doubles food use
- Heal only if not too cold or hot / heal faster the better temperature 
- Starving or very bad temperature causes damage (especiallt on the far temperature edges)

Objects:
- Cooked Rabbit Food Value: 10 --> 18
- Turkey Slice on Plate Food Value: 17 --> 24
- Omelette Food Value: 12 --> 15
- Cooked Goose: useChance = 0.3;
- Compost has less uses to make soil more rare, since water is more plenty: Soil (Wheat, Berry, Dung) + water ==> 4 Soil  

Ownership:
- Normal Gates are now inherited by a follower with the highest score or if none by the one the dead was following
- If none was there to inherit the gate the first finder will own the gate

Transitions:
- You can harvest Berries directly with a bowl
- Tule Stumps grow back after one hour 
- If you kill a Wolf you get meat (mutton) 
- You can cook meat (mutton) on hot coals

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
- If you hold any food the closest most YUM food for you is displayed
- YUM gives 3 food boni first eaten. 
- the first 3 times eaten a food is YUM
- food value is reduced minus one for each eaten / food has at least half the food value
    For example Berry Default 2: 5 YUM / 4 YUM / 3 YUM / 2 MEH / 1 MEH / 1 MEH / 1 MEH / 1 MEH ...
- if you eat YUM 1 pip is restored with a 0.8% chance from a random eaten food. If the random chosen food is the same food, no food value is restored
    For eample eats a Yum berry / then eats a YUM onion / 80%  x 50% = 40% chance that eating YUM onion restores 1 pip of berry food value 
    For eample eats a Yum berry / then eats a YUM onion / Then eats a YUM carrot / 80%  x 66%% = 52% chance that eating YUM carrot restores 1 pip of berry or onion food value   
- You can only eat if you have space for at least half the food value, not counting YUM or food reduction
- Extra food is stored as YUM Extra food
- YUM chain is used as Health // TODO change YUM --> Prestiege Max_Food --> Health 
- eating YUM increases YUM chain, while eating MEH reduces it

Generational Yum:
- a child inherits what is considered as YUM (the eaten food counts minus 0.2) from his gradma / mother in case there is none // TODO consider father

Food - Health - Hits
- YUM chain / Health influences now aging (0.5X to 2X) and speed (1.2X to 0.5X) / With this Berry munching is even worse
- Having less hitpoints / beeing more exhausted reduces speed  

Food - Cravings:
- Cravings display now a random food with a full YUM Boni, that has allready been eaten
- There is a Chance that a new complete never eaten radom food is chosen which is ID wise close to the last craved one
- Cravings give plus one extra food boni and YUM chain / Health which influences speed and aging

Animals:
- Animals can multiply and die
- Animals can leave their biome, but prefare to stay in own biome
- Stuff like trees and rabit holes wont block animal movement // TODO also for player / needs client support
- horses are the ultimative devourers you can eat anything from horse!
- Animals have more hit points so a shot / hit may or may not kill them

TODO Features: 
more up in the list means more likely to be implemented first but no gurantee!

- fathers
- better fences / allow to climb over fences, but get wounded / make destroyable if not protected by close gate)
- reconnection after server shutdown
- special stuff like hercules / snake master / super fast ager 
- job boni
- bridges
- horses that follow owner / horse ownership
- angry / bloody weapon if killing sheeps / animals
- animals grouping and hunting in winter
- better dogs
- advanced AI / AI villages
- princess / princess

TODO Missing:
more up in the list means more likely to be implemented first but no gurantee!

High priority:
- AI better escape that does not get stuck
- AI deadly animal fight
- AI player fight
- AI basic clothing
- Ai using containers needed also for player YUM display
- Ai ongoing longer distant movement
- AI tests with >20 AIs
- AI Stoneage

- TIME transitions with different endings like: Perhaps a Pumpkin

- COMBAT ill from moskitos
- COMBAT make smal wounds invisible if object held and allow to work (for example for poisone from snakes)

- BIRTH spawn eves on different locations
- BIRTH fathers

- MAP use bigger map (Europe?) / create tool to auto create map

- GATES Ally Gates and swing doors

- GRAVES graves with simple tools but more exhausting

Middle priority:
- CONNECTION saving of people data so that people can come back after server restart

- FIX Needle and Thread --> Bone Needle (How does this on earth work???)

- BIOME biome boni for different races
- BIOME change biome speed for specialists
- BIOME no dropping of stuff on water
- BIOME biome locked planting

- ALLY change ally color if leader changes
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
- TIME decay floors
- TIME support multiple time transitions see Fishing Pole --> store in ObjectData 

- ANIMAL animal movement on top of items
- ANIMAl eating sheep / meat transtions 
- ANIMAl chasing players

- SCORE give prestige boni for clothing / crown

- TEMPERATURE indoor temperature calculation 

- WINTER change snow biome / snow banks / melting if snow

- DONE??? do pattern transitions right like rubber horsecart pickup 

- AI Pottery
- AI Fire
- AI Advanced crafting
- AI Farming
- AI IRON AGE
- AI guards 

- WRITING / READING

- RAILS implement rails

- GATES visitor at gate

- YUM use yum category // for example for berry / berry bowl (maybe not needed?)

- CRAVING better order of craving list (currently its just ordered accoring to obj ID)

- CONNECTION twinning

- STATS implement basic server stats

- POINTER bell tower ringing
- POINTER maps / maps on waystone
- POINTER home marker

- LANGUAGE implement different languages

- ADVANED oil stuff, pipes, sprinkler, plow, roadmaker
- TOOLS use list of learned tool and set held_learned only true if tool is learned 

Low priority stuff:
- MAP calculate distance right when on end of map 
- radio
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


How to setup:
- TODO how to load new ObjectData from Repository
- there is a ServerSettings.txt file in SaveFiles folder. Settings are read at start and every 10 sec.
- server uses cached ObjectData to start fast
--> if you want to clear the cache you need to use another .\OneLifeData7\dataVersionNumber 
--> or delete the cache file file: .\SaveFiles\saveObjectData.bin
- you can use another map 
--> just override the current used one, or change the map file name in the Server ServerSettings.txt
- every 10 sec the server state is saved in the SaveFiles directory 
- every 8 hours a backup is made which is cycled through (up to 90 backups) / this can also be changed in ServerSettings.txt
--> the backups are in the folders: SaveFiles/1/ SaveFiles/2/.... SaveFiles/90/ 

For debugging:
if AllowDebugCommmands = true in ServerSettings.txt:
    can create objects with saying "create ID" / "create object" "create object!" with ! indicating that object ends with "object" or test wounds with using "hit" or "heal"
    --> for example: "create ore!" creates iron ore. "create iron" creates the first object with iron in the name.

How to Push stuff to git:
git push -u origin master
git push -u https://github.com/PXshadow/OpenLife

How to pull stuff from git:
git pull https://github.com/PXshadow/OpenLife

How to connect to the test server:
Currently the test server is run By PX and is running a very very old version of the server. I will inform if there is a new version which is testable.
ip: thinqbator.app
port: 8005

Jason server:
server10.onehouronelife.com
