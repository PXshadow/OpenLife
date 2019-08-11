Console Guide
=============
*Authored by PXshadow*

Console interperts the haxe programming language with the library Hscript.

- [Learn Haxe](https://code.haxe.org/)
- [Learn Hscript](https://github.com/HaxeFoundation/hscript/blob/master/README.md)

To pull up the console press **TAB** Press **ENTER** to run the input.

There is a list of variables that you will have acess to modify from the client.
* [Math](https://api.haxe.org/Math.html) #Haxe based class
* [Std](https://api.haxe.org/Std.html) #Haxe based class
* Client #Socket connection to server
* Grid #Number = 128
* Console #Console display object
* Util #Helpful functons
* Player #Main Player
* Program #Macro class that deals with AI
* Game #Main display class that holds everything game releated

Client
======
Send a message to the server
``` haxe
client.send("KA 0 0 ");
```
Close connection
``` haxe
client.close();
```
Set connection
``` haxe
client.ip = "127.0.0.1";
client.port = 2000;
```
Connect
``` haxe
client.connect();
```
Console
=======
console is a [DisplayObject](https://api.openfl.org/openfl/display/DisplayObject.html)
``` haxe
console.alpha = 0.5; //make transparent
```
input is a Text class
``` haxe
console.input.color = 0x00FF00; //green input 
```
Util
====
Shaders (dot,film,hex)
``` haxe
game.shaders = Util.shader("dot");
```

Player
======
player is a [DisplayObject](https://api.openfl.org/openfl/display/DisplayObject.html)
``` haxe 
player.rotation = 90; //rotate player 90 degrees
player.scaleX *= -1; //flip
```
Modify instance data of player
``` haxe
player.instance.data = 30; //set age for data
player.age(); //run aging system
```
Move player
```haxe 
player.step(0,1);//player step up
```

Program
=======
Declaritive syntax for client macros.
``` haxe
program.apply("trees",{alpha:0.5}); //get all trees and turn transparent
program.find("berry bush")path().use(); //find optimal berry bush and grab from it
```

Game
====
game is a [DisplayObject](https://api.openfl.org/openfl/display/DisplayObject.html)
``` haxe
game.rotation = 45; //tilt game 45 degrees
```
data
``` haxe
game.data.map.biome[0][0]; //bottom left map biome 0-7
game.data.map.blocking.get("0.0") //location is walking blocked, true = true, null = false
```
scaling
```haxe 
game.scale = 0.5; //half the scale of default game
```
objects is a [DisplayObject](https://api.openfl.org/openfl/display/DisplayObject.html) and [Tilemap](https://api.openfl.org/openfl/display/Tilemap.html)
``` haxe
game.objects.numTiles; //number of object tiles
game.objects.x += 10; //move objects to the right 10 pixels
```
ground is a [DisplayObject](https://api.openfl.org/openfl/display/DisplayObject.html) and [Shape](https://api.openfl.org/openfl/display/Shape.html)
``` haxe
game.ground.width = 10; //change ground width to 10 pixels
//draw white 100 pixel square top left
game.graphics.beginFill(0xFFFFFF);
game.graphics.drawRect(0,0,100,100);
```



