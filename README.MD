<p align="left"><img src="logo.png"/></p>

[![badge](https://img.shields.io/discord/595575978290446361?style=plastic)](https://discordapp.com/invite/gwghtky)
![Stars](https://img.shields.io/github/stars/pxshadow/openlife?style=social)


[![badge](badge.svg)](https://haxe.org)
[![forthebadge](https://forthebadge.com/images/badges/for-sharks.svg)](https://forthebadge.com)

Summary
======

*Open Life is a [Haxe](https://haxe.org) based OneLife engine. (the server is in beta the client is in early alpha state)*
* **Easy creation:** Build a client/bot (server in the future) easily for any OneLife server supported protocol (OneLife and TwoLife etc)

* **Open Api:**  Quickly search through the [api](https://pxshadow.github.io/OpenLife-Docs/api/index.html) and find variables and methods you need to create a new project.

* **Scripting:** Allows programmers to easily build programs on top of the engine architecture, using an event model. Can also be interpreted at runtime using the [hscript library](https://github.com/HaxeFoundation/hscript).

* **Easy to use build tools:** Unlike c++ the build tools are unified. A simple npm install will download the needed haxe language version, then download vscode download the haxe extension and Hashlink debugger and you can easily compile and debug the project :) (Currently will also need to manually setup Hashlink until it's integrated into lix)


* **Cross compiled languages:** Compiles to [Java](https://haxe.org/documentation/platforms/java.html), [C#](https://haxe.org/documentation/platforms/csharp.html), [Python](https://haxe.org/documentation/platforms/python.html), [Lua](https://haxe.org/manual/target-lua-getting-started.html), [Php](https://haxe.org/documentation/platforms/php.html), [Javascript/NodeJS](https://haxe.org/documentation/platforms/javascript.html)

* **Thoughtful, concise and efficient code:** Codebase is designed around ease of use and collaboration and continues to become as minimalis as possible while also keeping the level of abstraction low.

UPDATE: (Arcurus)
Currenty the project has three parts. A server, a client and a relay (headless client)
* The server.hxml project is a reimplementation of vanilla server with lot of changes compared to vanilla. It is fully playable and in BETA. 
The server / Open Life Reborn can run around 50 AIs which are able to run a village inclusive composting and some basic smithing
How to set the server up plus a complete change log you can find here: 
https://github.com/PXshadow/OpenLife/blob/master/TODO.MD/
* The client.hxml is currently very basic and can load all the needed stuff to do animations and connect to the server but does not yet send player commands to the server
* The relay (app.hxml?) can listen to any vanilla Client commands and send them to the server and the other way round. Here you can add your client based AI / It currently needs some little work to run again since focus was on implementing a server AI

The server AI uses an Interface to abstract if it runs on client or server but currently would need some work to be adapted to run client side.

 
