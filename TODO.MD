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
DONE: Generate limited map using image color as index for biome id
DONE: Made map "round", so you can walk over both x,y borders and come out on the other side. 
1. Map generates on the fly objects
2. Player can interact with objects (use,drop,remove,self(eat),move), map persistance as part of it. (Use our own data models)
3. set up transition system for object using/delay time example: stone + bighardrock -> sharp rock,  milkweed flowering -> 15 seconds -> milkweed fruiting