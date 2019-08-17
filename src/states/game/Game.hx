package states.game;

import openfl.geom.Rectangle;
import data.ChunkData.Chunk;
import lime.app.Future;
import data.Point;
import openfl.display.Shape;
import motion.easing.Quad;
import motion.Actuate;
import data.ObjectData.ObjectType;
import haxe.ds.Vector;
import data.PlayerData.PlayerType;
import console.Program;
import console.Console;
import data.MapData;
import data.MapData.MapInstance;
import data.PlayerData.PlayerInstance;
import data.PlayerData.PlayerMove;
import data.GameData;
import client.ClientTag;
import haxe.io.Bytes;

#if openfl
import openfl.display.FPS;
import openfl.display.DisplayObject;
import openfl.ui.Keyboard;
import openfl.events.MouseEvent;
import ui.Text;
import settings.Bind;
import openfl.display.Tile;
import openfl.display.Bitmap;
#end

class Game #if openfl extends states.State #end
{
    #if openfl
    public var draw:Draw;
    public var objects:Objects;
    public var select:Shape;
    public var selectX:Int = 0;
    public var selectY:Int = 0;
    public var bitmap:Bitmap;
    public var cameraSpeed:Float = 10;
    //text
    public var text:Text;
    #end
    var playerInstance:PlayerInstance;
    public var mapInstance:MapInstance;
    var index:Int = 0;
    public var data:GameData;
    var compress:Bool = false;
    var inital:Bool = true;
    public var program:Program;
    public function new()
    {
        //delelerative syntax for program console
        program = new Program(this);
        //set interp
        Console.interp.variables.set("game",this);
        Console.interp.variables.set("program",program);

        #if openfl
        super();
        stage.color = 0xFFFFFF;
        objects = new Objects(this);
        //tile selector
        select = new Shape();
        select.cacheAsBitmap = true;
        select.graphics.lineStyle(2,0xB7B7B7);
        select.graphics.drawRect(0,0,Static.GRID,Static.GRID);

        draw = new Draw(this);
        addChild(select);
        addChild(objects);
        Main.screen.addChild(draw);
        text = new Text();
        text.cacheAsBitmap = false;
        Main.screen.addChild(text);
        bitmap = new Bitmap();
        addChild(bitmap);
        #end
        //setup data
        data = new GameData(#if openfl objects.group#end);
        //connect
        if (true)
        {
            connect();
        }else{
            #if openfl
            //player
            setPlayer(cast(objects.add(19,true),Player));
            Player.main.instance = new PlayerInstance([]);
            Player.main.instance.move_speed = 3;
            data.playerMap.set(0,Player.main);
            //bush
            objects.add(30);
            //sheep
            objects.add(575);
            //trees
            objects.add(65);
            objects.add(2454);
            objects.add(49);
            objects.add(530);
            //spring
            objects.add(3030);
            #end
        }
    }
    //client events
    #if openfl
    var xs:Int = 0;
    var ys:Int = 0;
    override function update()
    {
        super.update();
        draw.update();
        //selector
        selectX = Std.int(stage.mouseX - objects.group.x);
        selectY = Std.int(stage.mouseY - objects.group.y);
        //set local for render
        
        //set global

        text.text = "num " + objects.group.numTiles;

        //player movement
        if(Player.main != null)
        {
            xs = 0;
            ys = 0;
            if (Bind.playerUp.bool) ys += 1;
            if (Bind.playerDown.bool) ys += -1;
            if (Bind.playerLeft.bool) xs += -1;
            if (Bind.playerRight.bool) xs += 1;
            if (xs != 0 || ys != 0) 
            {
                Player.main.goal = false;
                program.setup = false;
                Player.main.step(xs,ys);
            }
        }
        //update players
        //players
        it = data.playerMap.iterator();
        while(it.hasNext())
        {
            it.next().update();
        }
        //set camera to middle
        if (Player.main != null)
        {
            objects.group.x = lerp(objects.group.x,-Player.main.x * objects.scale + Main.setWidth/2 ,0.03);
            objects.group.y = lerp(objects.group.y,-Player.main.y * objects.scale + Main.setHeight/2,0.03);
        }
    }
    public inline function lerp(v0:Float,v1:Float,t:Float)
    {
        return v0 + t * (v1 - v0);
    }
    override function keyDown() 
    {
        super.keyDown();
        if (Bind.zoomIn.bool) zoom(1);
        if (Bind.zoomOut.bool) zoom(-1);
        if (Bind.playerSelf.bool)
        {
            program.self();
        }
        if (Bind.playerDrop.bool)
        {
            program.drop(selectX,selectY);
        }
        if (Bind.playerUse.bool)
        {
            program.use(selectX,selectY);
            Player.main.hold();
        }
    }
    #if openfl
    var it:Iterator<Player>;
    #end
    override function mouseDown() 
    {
        super.mouseDown();
        //fix crash
        if (Player.main == null) return;
        if (Bind.playerMove.bool)
        {
            program.path(selectX,selectY);
        }else{
            if (Bind.playerKill.bool) 
            {
                trace("kill");
                program.kill(selectX,selectY);
            }else{
                //use action if within range
                program.use(selectX,selectY);
            }
        }
    }
    override function mouseRightDown()
    {
        super.mouseRightDown();
        if (Player.main != null)
        {
            trace("oid " + Player.main.instance.o_id);
            if (Player.main.instance.o_id > 0)
            {
                program.drop(selectX,selectY);
            }else{
                program.remove(selectX,selectY);
            }
        }
    }
    public function move(x:Float=0,y:Float=0)
    {
        //flip
        x *= -1;
        y *= -1;
        //move
        objects.x += x;
        objects.y += y;
    }
    override function mouseScroll(e:MouseEvent) 
    {
        super.mouseScroll(e);
        zoom(e.delta);
    }
    public function zoom(i:Int)
    {
        if (objects.scale > 2 && i > 0 || objects.scale < 0.2 && i < 0) return;
        objects.scale += i * 0.08;
    }
    public function mapUpdate() 
    {
        trace("MAP UPDATE");
        //inital set camera
        if (inital)
        {
            objects.group.x = -data.map.x * Static.GRID;
            objects.group.y = -data.map.y * Static.GRID;
            inital = false;
        }
        var chunk = data.chunk.add(mapInstance.x,mapInstance.y,mapInstance.width,mapInstance.height);
        //out of range chunks and add overlap
        trace("chunk length " + data.chunk.array.length);
        var out:Float = 32;
        var dis:Float = 0;
        var overlaps:Array<Rectangle> = [];
        var rect:Rectangle;
        var c:Chunk;
        for (i in 0...data.chunk.array.length - 3)
        {
            c = data.chunk.array[i];
            if (c == null) continue;
            //out of range
            dis = Math.sqrt(Math.pow(chunk.centerY - c.centerY,2) + Math.pow(chunk.centerX - c.centerX,2));
            if (dis > out)
            {
                trace("remove chunk " + i + " dis " + dis);
                data.chunk.remove(c);
            }
            //overlap
            rect = new Rectangle(chunk.x,chunk.y,chunk.width,chunk.height).intersection(new Rectangle(c.x,c.y,c.width,c.height));
            if (rect.width > 0)
            {
                trace("overlap " + rect);
                overlaps.push(rect);
            }
        }
        //draw chunks
        draw.graphics.clear();
        for (c in data.chunk.array)
        {
            draw.graphics.endFill();
            draw.graphics.lineStyle(2,0);
            draw.graphics.beginFill(0,0.2);
            draw.graphics.drawRect(Main.setWidth/2 + c.x * 4,Main.setHeight/2 + c.y * 4,c.width * 4,c.height * 4);
        }
        var skip:Bool = false;
        for(j in chunk.y...chunk.y + chunk.height)
        {
            //overlap checker
            for (i in chunk.x...chunk.x + chunk.width)
            {
                skip = false;
                for (overlap in overlaps)
                {
                    if (overlap.contains(i,j))
                    {
                        skip = true;
                        break;
                    }
                }
                if (skip) continue;
                //add tiles
                chunk.floor.set(i,j,[]);
                chunk.object.set(i,j,[]);
                //floor
                if (!objects.add(data.map.floor.get(i,j),i,j))
                {
                    //add ground as there is no floor
                    objects.addGround(data.map.biome.get(i,j),i,j);
                }
                //object
                objects.add(data.map.object.get(i,j),i,j);
            }
        }
    }
    #end
    
    public function end()
    {
        switch(Main.client.tag)
        {
            case PLAYER_UPDATE:
            #if openfl
            if (Player.main == null) 
            {
                setPlayer(objects.player);
                Player.main.sort();
            }
            objects.player = null;
            #end
            Main.client.tag = null;
            default:
        }
    }
    public function setPlayer(player:Player)
    {
        Player.main = player;
        Console.interp.variables.set("player",Player.main);
    }
    public function disconnect()
    {
        Main.client.close();
        reset();
    }
    public function reset()
    {
        Player.main = null;
        objects.removeTiles();
        inital = true;
    }
    override function resize() 
    {
        super.resize();
        objects.x = -Main.screen.x * 1/Main.scale;
        objects.y = -Main.screen.y * 1/Main.scale;
        objects.width = stage.stageWidth * 1/Main.scale;
        objects.height = stage.stageHeight * 1/Main.scale;
    }
    public function connect()
    {
        Main.client.login.accept = function()
        {
            trace("accept");
            //set message reader function to game
            Main.client.message = message;
            Main.client.end = end;
            //Main.client.login = null;
            Main.client.tag = null;
            index = 0;
        }
        Main.client.login.reject = function()
        {
            trace("reject");
            //Main.client.login = null;
        }
        Main.client.message = Main.client.login.message;
        trace("connect " + Main.client.ip + " email " + Main.client.login.email);
        Main.client.connect();
    }
    public function message(input:String) 
    {
        switch(Main.client.tag)
        {
            case PLAYER_UPDATE:
            playerInstance = new PlayerInstance(input.split(" "));
            #if openfl
            objects.addPlayer(playerInstance);
            #end
            case PLAYER_MOVES_START:
            var playerMove = new PlayerMove(input.split(" "));
            if (data.playerMap.exists(playerMove.id))
            {
                playerMove.movePlayer(data.playerMap.get(playerMove.id));
            }
            Main.client.tag = null;
            case MAP_CHUNK:
            trace("MAP CHUNK");
            if(compress)
            {
                Main.client.tag = null;
                data.map.setRect(mapInstance.x,mapInstance.y,mapInstance.width,mapInstance.height,input);
                #if openfl
                mapUpdate();
                #end
                //mapInstance = null;
                //toggle to go back to istance for next chunk
                compress = false;
            }else{
                var array = input.split(" ");
                //trace("map chunk array " + array);
                for(value in array)
                {
                    switch(index++)
                    {
                        case 0:
                        mapInstance = new MapInstance();
                        mapInstance.width = Std.parseInt(value);
                        trace("width " + mapInstance.width);
                        case 1:
                        mapInstance.height = Std.parseInt(value);
                        case 2:
                        mapInstance.x = Std.parseInt(value);
                        case 3:
                        mapInstance.y = Std.parseInt(value);
                        case 4:
                        mapInstance.rawSize = Std.parseInt(value);
                        case 5:
                        mapInstance.compressedSize = Std.parseInt(value);
                        //set min
                        if (data.map.x > mapInstance.x) data.map.x = mapInstance.x;
                        if (data.map.y > mapInstance.y) data.map.y = mapInstance.y;
                        if (data.map.width < mapInstance.x + mapInstance.width) data.map.width = mapInstance.x + mapInstance.width;
                        if (data.map.height < mapInstance.y + mapInstance.height) data.map.height = mapInstance.y + mapInstance.height;
                        trace("map chunk " + mapInstance.toString());
                        index = 0;
                        //set compressed size wanted
                        Main.client.compress = mapInstance.compressedSize;
                        compress = true;
                    }
                }
            }
            case MAP_CHANGE:
            //x y new_floor_id new_id p_id optional oldX oldY playerSpeed
            var change = new MapChange(input.split(" "));
            #if openfl
            var tile:Tile;
            if (change.speed > 0)
            {
                //move object 
            }else{
                var type:ObjectType = change.floor > 0 ? FLOOR : OBJECT;
                var id = type == FLOOR ? change.floor : change.id;
                //remove object regardless
                /*for (i in 0...objects.numTiles)
                {
                    tile = objects.group.getTileAt(i);
                    if (change.x == tile.data.x && change.y == tile.data.y && type == tile.data.type)
                    {
                        objects.group.removeTile(tile);
                        break;
                    }
                }
                if (id > 0)
                {
                    //add new object to map
                    objects.add(id,change.x,change.y,false);
                }*/
            }
            #end
            //change data todo:

            Main.client.tag = null;
            index = 0;
            case HEAT_CHANGE:
            //trace("heat " + input);
            Main.client.tag = null;
            index = 0;
            case FOOD_CHANGE:
            trace("food change " + input);
            //also need to set new movement move_speed: is floating point playerSpeed in grid square widths per second.
            case FRAME:
            Main.client.tag = null;
            index = 0;
            case PLAYER_SAYS:
            trace("player say " + input);
            #if openfl
            draw.say(input);
            #end
            case PLAYER_OUT_OF_RANGE:
            //player is out of range

            case LINEAGE:
            //p_id mother_id grandmother_id great_grandmother_id ... eve_id eve=eve_id

            case NAME:
            //p_id first_name last_name last_name may be ommitted.

            case DYING:
            //p_id isSick isSick is optional 1 flag to indicate that player is sick (client shouldn't show blood UI overlay for sick players)

            case HEALED:
            //p_id player healed no longer dying.

            case MONUMENT_CALL:
            //MN x y o_id monument call has happened at location x,y with the creation object id

            case GRAVE:
            //x y p_id

            case GRAVE_MOVE:
            //xs ys xd yd swap_dest optional swap_dest parameter is 1, it means that some other grave at  destination is in mid-air.  If 0, not

            case GRAVE_OLD:
            //x y p_id po_id death_age underscored_name mother_id grandmother_id great_grandmother_id ... eve_id eve=eve_id
            //Provides info about an old grave that wasn't created during your lifetime.
            //underscored_name is name with spaces replaced by _ If player has no name, this will be ~ character instead.

            case OWNER_LIST:
            //x y p_id p_id p_id ... p_id

            case VALLEY_SPACING:
            //y_spacing y_offset Offset is from client's birth position (0,0) of first valley.

            case FLIGHT_DEST:
            //p_id dest_x dest_y
            trace("FLIGHT FLIGHT FLIGHT " + input.split(" "));
            default:
        }
    }
}