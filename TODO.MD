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

SPEED:
- different speed for different biomes
- limited speed for lower age < 10 and age > 55
- starving to death reduces speed X0.5 if not in car or on horse

Food - YUM:
- YUM gives flat 2X food boni
- MEH reduces food value minus one for each eaten / For example Berry Default 3: 6 / 2 / 1 / 1 / 1 ...
- You can only eat if you have space for at least half the food value, not counting yum.
- Extra food is stored as YUM Extra food
- YUM chain is used as SCORE  (SOON) (Not how long you live is important but how you live is important)


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





