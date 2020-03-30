package game;
import client.Client;
import data.map.MapData;
import data.GameData;
#if openfl
import openfl.events.Event;
import motion.easing.Sine;
import openfl.geom.Point;
import openfl.display.TileContainer;
import motion.easing.Quad;
import motion.easing.Linear;
import motion.MotionPath;
import motion.Actuate;
import openfl.display.Tile;
#end
import data.object.player.PlayerInstance;
import haxe.Timer;
import data.object.SpriteData;
import data.object.ObjectData;
import data.animation.AnimationData;
import haxe.ds.Vector;
#if nativeGen @:nativeGen #end
class Player #if openfl extends TileContainer #end
{
    //statics
    #if visual
    private static inline var babyHeadDownFactor:Float = 0.6;
    private static inline var babyBodyDownFactor:Float = 0.75;
    private static inline var oldHeadDownFactor:Float = 0.35;
    private static inline var oldHeadForwardFactor:Float = 2;
    #end
    #if openfl
    public var objects:game.Objects;
    public var heldObject:TileContainer;
    //clothing hat;tunic;front_shoe;back_shoe;bottom;backpack
    public var clothing:Vector<Array<Tile>>;
    public var _sprites:Array<Tile> = [];
    #end
    public var instance:PlayerInstance;
    var clothingInt:Array<Int> = [0,0,0,0,0];
    //pathing
    public var lastMove:Int = 1;
    public var moveTimer:Timer;
    public var moving:Bool = false;
    public var goal:Bool = false;
    public var follow:Bool = true;
    var multi:Float = 1;
    //locally used instance pos
    public var ix:Int = 0;
    public var iy:Int = 0;
    //locally used object
    public var oid:Array<Int> = [];
    public var held:Bool = false;
    public var ageInt:Int = 0;
    //name
    public var firstName:String = "";
    public var lastName:String = "";
    public var text:String = "";
    public var inRange:Bool = true;
    //main player
    public var main:Bool = false;
    public function new()
    {
        #if openfl
        super();
        #end
    }
    #if openfl
    public function getAgeHeadOffset(inAge:Float,head:Point,body:Point,frontFoot:Point)
    {
        if (inAge == -1) return new Point();
        var maxHead = head.y - body.y;
        if (inAge < 20)
        {
            var yOffset = ( ( 20 - inAge ) / 20 ) * babyHeadDownFactor * maxHead;
            return new Point(0,Math.round(-yOffset));
        }
        if (inAge >= 40)
        {
            if (inAge > 60)
            {
                inAge = 60;
            }
            var vertOffset = ( ( inAge - 40) / 20 ) * oldHeadDownFactor * maxHead;
            var footOffset = frontFoot.x - head.x;
            var forwardOffset = ( ( inAge - 40 ) / 20 ) * oldHeadDownFactor * footOffset;
            return new Point(Math.round(forwardOffset),Math.round(-vertOffset));
        }
        return new Point();
    }
    public function getAgeBodyOffset(inAge:Float,pos:Point)
    {
        if (inAge == -1) return new Point();
        if (inAge < 20)
        {
            var maxBody = pos.y;
            var yOffset = ( ( 20 - inAge) / 20) * babyBodyDownFactor * maxBody;
            return new Point(0,Math.round(-yOffset));
        }
        return new Point();
    }
    public function computePathSpeedMod():Float
    {
        var floorData = Game.data.objectMap.get(Game.data.map.floor.get(ix,iy));
        var multiple:Float = 1;
        if (floorData != null) multiple *= floorData.speedMult;
        if (oid.length == 0) return multiple;
        var objectData = Game.data.objectMap.get(oid[0]);
        if (objectData != null) multiple *= objectData.speedMult;
        return multiple;
    }
    public function measurePathLength():Float
    {
        var diagLength:Float = 1.4142356237;
        var totalLength:Float = 0;
        if (moves.length < 2)
        {
            return totalLength;
        }
        var lastPos = moves[0];
        for (i in 1...moves.length)
        {
            if (moves[i].x != lastPos.x && moves[i].y != lastPos.y)
            {
                totalLength += diagLength;
            }else{
                //not diag
                totalLength += 1;
            }
            lastPos = moves[i];
        }
        return totalLength;
    }
    #end
    public function update()
    {
        
    }
    public function force() 
    {
        //moves = [];
        #if openfl
        Actuate.pause(this);
        //local position
        x = instance.x * Static.GRID;
        y = (Static.tileHeight - instance.y) * Static.GRID;
        #end
        if (main)
        {
            Game.client.send("FORCE");
        }
    }
    public function set(data:PlayerInstance)
    {
        if (instance == null)
        {
            ix = data.x;
            iy = data.y;
        }
        instance = data;
        //pos and age
        //trace("num " + instance.done_moving_seqNum);
        if (instance.forced || instance.done_moving_seqNum > 0) 
        {
            trace("force!");
            if (held)
            {
                //added back to stage
                #if openfl
                parent.addTile(this);
                //Main.objects.group.addTile(this);
                #end
                held = false;
            }
            ix = instance.x;
            iy = instance.y;
            //force movement
            force();
        }
        //remove moves
        var data = Game.data.objectMap.get(instance.po_id);
        #if openfl
        if (ageInt != Std.int(instance.age)) 
        {
            ageInt = Std.int(instance.age);
            age(data);
        }
        #end
        hold();
        cloths(data);
    }
    #if openfl
    public function age(data:ObjectData)
    {
        trace("TODO setup age");
        return;
        var ageInital:Bool = true;
        var headMoveX:Float = 0;
        var headMoveY:Float = 0;
        var bodyMoveX:Float = 0;
        var bodyMoveY:Float = 0;
        //redering
        ageInt = Std.int(instance.age);
        objects.visibleSprites(instance.po_id,sprites(),ageInt);
        //get and set sprites for head
        var head = _sprites[data.headIndex];
        objects.setSprite(head,data.spriteArray[data.headIndex],0,0);
        var body = _sprites[data.bodyIndex];
        objects.setSprite(body,data.spriteArray[data.bodyIndex],0,0);
        var frontFoot = _sprites[data.frontFootIndex];
        objects.setSprite(frontFoot,data.spriteArray[data.frontFootIndex],0,0);
        //points
        var headPos = new Point(head.x,head.y);
        var bodyPos = new Point(body.x,body.y);
        var frontFootPos = new Point(frontFoot.x,frontFoot.y);

        headPos = headPos.add(getAgeHeadOffset(instance.age,headPos,bodyPos,frontFootPos));
        headPos = headPos.add(getAgeBodyOffset(instance.age,bodyPos));

        bodyPos = bodyPos.add(getAgeBodyOffset(instance.age,bodyPos));

        //diffrence move children now
        var dx:Float = 0;
        var dy:Float = 0;
        for (i in 0...data.numSprites)
        {
            if (data.spriteArray[i].parent == data.headIndex)
            {
                _sprites[i].x += headPos.x - head.x - headMoveX;
                _sprites[i].y += headPos.y - head.y - headMoveY;
                continue;
            }
            if (data.spriteArray[i].parent == data.bodyIndex)
            {
                _sprites[i].x += bodyPos.x - body.x - bodyMoveX;
                _sprites[i].y += bodyPos.y - body.y - bodyMoveY;
            }
        }
        headMoveX = headPos.x - head.x;
        headMoveY = headPos.y - head.y;
        bodyMoveX = bodyPos.x - body.x;
        bodyMoveY = bodyPos.y - body.y;
        //set body and head
        body.x = bodyPos.x;
        body.y = bodyPos.y;

        head.x = headPos.x;
        head.y = headPos.y;
    }
    #end
    public function cloths(data:ObjectData)
    {
        #if !openfl
        clothingInt = MapData.id(instance.clothing_set,";",",");
        #else
        var temp:Array<Int> = MapData.id(instance.clothing_set,";",",");
        if (!Static.arrayEqual(temp,clothingInt))
        {
            //clean clothing
            if (clothing != null) for (piece in clothing)
            {
                if (piece != null) for (sprite in piece)
                {
                    removeTile(sprite);
                }
            }
            //set new
            clothingInt = temp;
            var sprites:Array<Tile> = [];
            var clothsData:ObjectData;
            var offsetX:Float = 0;
            var offsetY:Float = 0;
            var index:Int = 0;
            var i:Int = 0;
            var place:Int = 0;
            clothing = new Vector<Array<Tile>>(5 + 1);
            for (id in clothingInt)
            {
                if (id > 0)
                {
                    switch (i++)
                    {
                        case 0: 
                        //hat (slight hack set hat to the front of hair)
                        index = data.headIndex;
                        place = 0 + 20;
                        case 1:
                        //tunic
                        index = data.bodyIndex;
                        place = 1 + 5;
                        case 2:
                        //front shoe
                        index = data.frontFootIndex;
                        place = 2;
                        case 3: 
                        //back shoe
                        index = data.backFootIndex;
                        place = 3;
                        case 4:
                        //bottom 
                        index = data.bodyIndex;
                        place = 4;
                        case 5:
                        //backpack
                        index = data.bodyIndex;
                        place = 5;
                    }
                    //clothing
                    clothsData = Game.data.objectMap.get(id);
                    offsetX = clothsData.clothingOffset.x + getTileAt(index).x;
                    offsetY = -clothsData.clothingOffset.y + getTileAt(index).y;
                    sprites = objects.create(clothsData,offsetX,offsetY,true);
                    clothing[i - 1] = sprites;
                    for (j in 0...sprites.length)
                    {
                        //addTile(sprites[j]);
                        addTileAt(sprites[j],index + place + i + j);
                    }
                }else{
                    //sub
                }
            }
        }
        #end
    }
    #if openfl
    public function emote(index:Int=-1)
    {
        if (index == -1) return;
        var emot = Game.data.emotes[index];
        var data = Game.data.objectMap.get(instance.po_id);
        if (data == null || emot == null) return;
        //data.
    }
    #end
    public function hold()
    {
        #if !openfl
        if (Static.arrayEqual(instance.o_id,oid))
        {
            //change
            oid = instance.o_id;
        }
        #else
        if (!Static.arrayEqual(oid,instance.o_id))
        {
            //check if was player to re add to stage
            if (instance.o_id.length == 1 && instance.o_id[0] < 0) objects.group.addTile(heldObject);
            //change
            oid = instance.o_id;
            trace("oid " + oid);
            removeTile(heldObject);
            if (oid.length == 0 || oid[0] == 0) return;
            //add
            if (oid[0] > 0)
            {
                heldObject = new TileContainer();
                trace("oid " + oid + " objects " + objects);
                objects.add(oid,0,0,heldObject);
            }else{
                heldObject = Game.data.playerMap.get(oid[0] * -1);
                if (heldObject == null) 
                {
                    trace("held baby not found " + oid[0]);
                    return;
                }
                objects.removeTile(heldObject);
            }
            //pos
            var data = objects.get(oid[0]);
            heldObject.x = data.heldOffset.x;
            heldObject.y = data.heldOffset.y;
            //data.held
            //add
            addTile(heldObject);
            Actuate.stop(heldObject);
            //Actuate.tween(heldObject,0.5,{x:instance.o_origin_x,y:-height/2 - instance.o_origin_y}).ease(Sine.easeInOut);
        }
        #end
    }
    #if openfl
    public function sprites():Array<Tile>
    {
        if (_sprites.length == 0)
        {
            for (i in 0...numTiles) _sprites.push(getTileAt(i));
            _sprites.remove(heldObject);
            trace("clothing " + clothing);
            if (clothing != null) for (array in clothing) if (array != null) for (cloths in array) _sprites.remove(cloths);
        }
        return _sprites;
    }
    var ageInital:Bool = true;
    var headMoveX:Float = 0;
    var headMoveY:Float = 0;
    var bodyMoveX:Float = 0;
    var bodyMoveY:Float = 0;
    /*public function age(data:ObjectData)
    {
        
        
    }*/
    #end
}