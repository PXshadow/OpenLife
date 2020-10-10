Frequently asked questions
============

* Question: How do changes made to data objects get sent to the client/server 

Answer: [protocol.txt](protocol.txt) documents how the messages are exchanged both from the client perspective and server side.

* Question: What is the structure for the project?

Answer: The project is broken up into 4 main parts client/data/server/engine

client + data + engine are used by the client and server + data are used by the server. 

## Client
 has the [Client.hx](openlife/client/Client.hx) class that holds the Socket and how it sends and responds to messages from the server. [ClientTag.hx](openlife/client/ClientTag.hx) holds an enum that connect to the tags the client receives from the server.

## Data
is a large package that stores all of the data classes for the game, including object/map/player data.

## Engine
 is a package used by the client only to allow easy extension/implementation of high level events and program functions. It turns the low level dealing with the data by the client into higher level processes for example 
from:  
```
tag = PLAYER_SAYS, 35/0 HELLO WORLD
``` 
to: 
```haxe 
playerSays(id:Int,curse:Bool,text:String)
```
for sending it also uses the same principle and uses the class [Program.hx](openlife/engine/Program.hx)
from:
```haxe
send("SAYS 0 0 HELLO WORLD");
```
to:
```haxe
program.say("HELLO WORLD");
```

## Server

is a package used only by the server to allow straight forward generation of the world and connection of hundreds of clients at once. It currently is only on it's fire iteration so things may vary alot and be refined so the api's are more unified from the server and the client. Right now there is a [ThreadServer.hx](openlife/server/ThreadServer.hx) that is the low level system that generates one thread per Socket connection and handles the updating. [ServerTag.hx](openlife/client/ServerTag.hx) holds an enum that connect to the tags the server receives from the client. [Map.hx](openlife/server/Map.hx) is responsible for the map data and generation (in the future it should probably be extended by data package's [MapData](openlife/data/map/MapData.hx)). Server holds the global functions that are independent of a single connection. [Connection.hx](openlife/server/Connection.hx) class has the functions that are run in reference to a single connection to the server.

* Question: When did this project start?

Answer: June 5th 2019 is the first github commit.

* Question: Why is this being worked on?

Answer: the reasons why have shifted throughout time but the core idea has remained, because it's worth perusing. New breakthroughs have lead to most of the reasons shifting and have also scaled the project immensely from a simple buggy visual Client, to a competitive alternative to OneLife's vanilla implementation designed as the best engine to power clients/bots/mods/servers in OneLife across programming languages.

* Question: what can you use this for?

Answer: the library core for a new modded client/client scripts/bots/modded server etc following the OneLife protocol. In any of these languages Haxe, Javascript, Java, Python, Lua, C++, C, C# etc.