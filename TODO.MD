How to connect to the test server:
ip: thinqbator.app
port: 8005

For debugging:
if AllowDebugCommmands = true in ServerSettings.txt:
    can create objects with saying "create ID" / "create object" "create object!" with ! indicating that object ends with "object" or test wounds with using "hit" or "heal"
    --> for example: "create ore!" creates iron ore. "create iron" creates the first object with iron in the name.

Summary: 
- The server is written from scratch in Haxe and in early ALPHA, so except errors / feel welcome to report them
- Limited hand made map which can be designed using a picture
- Rivers oceans and mountains that block the movement
- No locked iron or wells
- All natural stuff can re-spawn (exception well, iron and oil stuff)
--> So having a lucky spot of one thing keeps it lucky also in the future
- All But ancient stuff can decay (exception well, iron and oil stuff)
- New craving system that shows you what has full YUM boni
- Yum chain is now your prestiege (score) / health and influcences movement speed and aging
--> so yes in theory you can life longer then 60
- Quality of Life: You can pick berriers directly with a bowl, you can eat while on a horse. Shoes give extra speed
- Wells fill up at least 3 times faster



Not yet implemented:
- temperature (clothing does already reduce food use up to 66% - Jungle reduces food need 33%)
- combat
- player pickup / incarnation / family
- see more in TODO below

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


SERVER
======
MAP:
- Generate limited map using image color as index for biome id
- Made map "round", so you can walk over both x,y borders and come out on the other side. 
- Has rivers and oceans  
- Iron can be used by default 
- No dry spring and dry tarry spot
- no gradients
- no expert waystones


World:
- Paradise is back! Wild Berries and Bana come finaly back with time 
- objects respawn near original objects with a small chance
- objects except ancient, well, Iron Mine, Oil Pumps decay with a small chance
- decay chance is 10X reduced if on floor and 10X increeased if food

Healing
- max food store equals hitpoints
- player heals with time but healing doubles food use

Objects:
- Cooked Rabbit Food Value: 10 --> 18
- Turkey Slice on Plate Food Value: 17 --> 24
- Omelette Food Value: 12 --> 18
- Cooked Goose: useChance = 0.3;

Transitions:
- You can harvest Berries directly with a bowl
- Tule Stumps grow back after one hour 
- You can cook meat (mutton) on hot coals
- If you kill a Wolf you get meat (mutton) 

Aging:
- starving to death slows down aging below 14 and increases above  
- higher food use for age below 14 if not starving to death / they need to grow!

SPEED:
- different speed for different biomes
- limited speed for lower age < 10 and age > 55
- starving to death reduces speed X0.5 if not in car or on horse
- objects in container / sub container lower speed
- horses and cars and carts are slower in bad biomes
- road reduces speed mali of bad biome and heavy objects
- having both shoes give now X1.1 Speed

Food - YUM:
- YUM gives 3 food boni first eaten. 
- the first 3 times eaten a food is YUM
- food value is reduced minus one for each eaten / food has at least half the food value
    For example Berry Default 2: 5 YUM / 4 YUM / 3 YUM / 2 MEH / 1 MEH / 1 MEH / 1 MEH / 1 MEH ...
- if you eat YUM 1 pip is restored with a 0.8% chance from a random eaten food. If the random chosen food is the same food, no food value is restored
    For eample eats a Yum berry / then eats a YUM onion / 80%  x 50% = 40% chance that eating YUM onion restores 1 pip of berry food value 
    For eample eats a Yum berry / then eats a YUM onion / Then eats a YUM carrot / 80%  x 66%% = 52% chance that eating YUM carrot restores 1 pip of berry or onion food value   
- You can only eat if you have space for at least half the food value, not counting YUM or food reduction
- Extra food is stored as YUM Extra food
- YUM chain is used as Health 
- eating YUM increases YUM chain, while eating MEH reduces it

Food - Health
- YUM chain / Health influences now aging (0.5X to 2X) and speed (1.2X to 0.5X) / With this Berry munching is even worse

Food - Cravings:
- Cravings display now a random food with a full YUM Boni, that has allready been eaten
- There is a Chance that a new complete never eaten radom food is chosen which is ID wise close to the last craved one
- Cravings give plus one extra food boni and YUM chain / Health which influences speed and aging

Animals:
- Animals can multiply and die
- Animals can leave their biome, but prefare to stay in own biome
- Stuff like trees and rabit holes wont block animal movement // TODO also for player
- horses are the ultimative devourers you can eat anything from horse!

TODO Features: 
more up in the list means more likely to be implemented first but no gurantee!
- Winter / seasons / ice ages
- horses that follow owner
- princess / princess / nobles
- reconnection after server shutdown
- special stuff like hercules / snake master / super fast ager 
- job boni
- bridges

TODO Missing:
more up in the list means more likely to be implemented first but no gurantee!
- jump out from player holding
- support multiple time transitions see Fishing Pole --> store in ObjectData 
- wiggle command
- animal interactions / collisions
- give more space for cities in the map
- wounds / healing part 2
- combat / generally allowed
- better temperature calculation
- change biome speed for specialists
- send player out of range update
- give option to use normal YUM mechanics
- better map
- decay contained stuff in graves / generally decay stuff in containers
- decay use item for decay like trash
- decay floors
- death while holding some one, or riding / driving something or using cart
- reconnection
- incarnation
- multiplayer testing
- biome locked planting
- no dropping of stuff on water
- predefined naming
- time transitions for objects in container
- do pattern transitions right like rubber horsecart pickup 
- biome boni for different races
- leadership
- rails 
- faces / for example hungry / ill
- ill from moskitos
- temperature / fire / heat objects / indoor / different biomes are missing
- server stats
- curse system replaced through grave system / in short if your old body is rottening nearby you have a hard time
- fences / gates
- score / score boni
- families
- / commands 
- leader commands
- maps / maps on waystone
- yum category // for example for berry / berry bowl (maybe not needed?)
- better order of craving list (currently its just ordered accoring to obj ID)
- twinning
- different languages
- bell tower ringing


Low priority stuff:
- advanced transitions like oil stuff, sprinkler, cars
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