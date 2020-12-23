TODO
====
1. Transition history system (used by both bot and server)
2. Implement another path finding algorithm
3. Algorithm to score tech tree climb, time dependent, and based on future potential for survival as well
4. Solo Eve Bot run, scored after 1 life and world is static and reset each time
5. Task Queue system with Auto.hx class
6. Algorithm to identify what is needed next for overseer



SERVER
======
MAP:
- Generate limited map using image color as index for biome id
- Made map "round", so you can walk over both x,y borders and come out on the other side. 
- Has rivers and oceans  
- Iron can be used by default 
- No dry spring and tarry spot
- no gradients
- no expert waystones

Transitions:
- Paradise is back! Wild Berries and Bana come finaly back with time 

Aging:
- starving to death slows down aging below 14 and increases above  
- double food use for age below 14 if not starving to death / they need to grow!

SPEED:
- different speed for different biomes
- limited speed for lower age < 10 and age > 55
- starving to death reduces speed X0.5 if not in car or on horse
- objects in container / sub container lower speed
- horses and cars and carts are slower in bad biomes

Food - YUM:
- YUM gives 3 food boni first eaten. 
- the first 3 times eaten a food is YUM
- food value is reduced minus one for each eaten / food has at least half the food value
    For example Berry Default 3: 6 YUM / 5 YUM / 4 YUM / 3 MEH / 2 MEH / 2 MEH / 2 MEH / 2 MEH ...
- if you eat YUM 1 pip is restored with a 0.8% chance from a random eaten food. If the random chosen food is the same food, no food value is restored
    For eample eats a Yum berry / then eats a YUM onion / 80%  x 50% = 40% chance that eating YUM onion restores 1 pip of berry food value 
    For eample eats a Yum berry / then eats a YUM onion / Then eats a YUM carrot / 80%  x 66%% = 52% chance that eating YUM carrot restores 1 pip of berry or onion food value   
- You can only eat if you have space for at least half the food value, not counting YUM or food reduction
- Extra food is stored as YUM Extra food
- YUM chain is used as Health 
- eating YUM increases YUM chain, while eating MEH reduces it
- TODO Eating YUM increases prestiege, eating MEH reduces it (Not how long you live is important but how you live is important)

Food - Health
- YUM chain / Health influences now aging (0.5X to 2X) and speed (1.2X to 0.5X) / With this Berry munching is even worse

Food - Cravings:
- Cravings display now a random food with a full YUM Boni, that has allready been eaten
- There is a Chance that a new complete never eaten radom food is chosen which is ID wise close to the last craved one
- Cravings give plus one extra food boni and YUM chain / Health which influences speed and aging


Animals:
- Animals can multiply
- Animals can leave their biome, but prefare to stay in own biome
- Stuff like trees and rabit holes wont block animal movement // TODO also for player
- horses are the ultimative devourers you can eat anything from horse!

TODO Features
- Winter
- Underground

TODO Missing:
- hard work
- speed stuff
- backpack
- use on table
- animal interactions / collisions
- yum category // for example for berry / berry bowl
- death
- world persistance 
- incarnation
- biome boni for different races
- feeding of other people
- leadership
- better order of craving list (currently its just ordered accoring to obj ID)
- faces / for example hungry / ill
- ill from moskitos
- temperature
- curse system replaced through grave system / in short if your old body is rottening nearby you have a hard time
- combat / generally allowed
- fences / gates
- wounds / healing
- epecial stuff like hercules / snake master / super fast ager 
- better map
- multiplayer testing
- score / score boni
- nobles
- job boni
- naming
- families
- commands 
- speech limit
- different languages


Low priority stuff:
- advanced transitions like oil stuff, sprinkler, cars
- radio
- photo







