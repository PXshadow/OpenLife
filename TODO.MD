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

Aging:
- starving to death slows down aging below 14 and increases above  
- double food use for age below 14 if not starving to death / they need to grow!

SPEED:
- different speed for different biomes
- limited speed for lower age < 10 and age > 55
- starving to death reduces speed X0.5 if not in car or on horse

Food - YUM:
- YUM gives 3 food boni first eaten. 
- the first 3 times eaten a food is YUM
- food value is reduced minus one for each eaten
    For example Berry Default 3: 6 YUM / 5 YUM / 4 YUM / 3 MEH / 2 MEH / 1 MEH / 1 MEH / 1 MEH ...
- if you eat YUM 1 pip is restored from a random eaten food. Zero are restored if random food is the current eaten food
    For eample eats a Yum berry / then eats a YUM onion / 50% chance that eating YUM onion restored berry to full food value / YUM    
- You can only eat if you have space for at least half the food value, not counting YUM or food reduction
- When food value is less then halve it can only be eaten if starving to death in this case the food has at least half the value
- Extra food is stored as YUM Extra food
- YUM chain is used as Health
- eating YUM increases YUM chain, while eating MEH reduces it
- TODO Eating YUM increases prestiege, eating MEH reduces it (Not how long you live is important but how you live is important)


Animals:
- Animals can multiply
- Animals can leave their biome, but prefare to stay in own biome
- Stuff like trees and rabit holes wont block animal movement // TODO also for player


TODO:
- hard work
- speed stuff
- backpack
- use on table
- animal interactions / collisions
- more food logic like food influencing speed and maxage
- yum category // for example for berry / berry bowl
- death
- world persistance 
- incarnation
- leadership
- cravings
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
- biome boni for different races
- feeding of other people
- naming
- families
- commands 
- speech limit
- different languages
- advanced transitions like oil stuff, sprinkler, cars





