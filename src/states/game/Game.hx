package states.game;

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
    var dialog:Dialog;
    public var ground:Ground;
    public var draw:Draw;
    public var objects:Objects;
    public var select:Shape;
    public var selectX:Int = 0;
    public var selectY:Int = 0;
    public var bitmap:Bitmap;
    public var cameraSpeed:Float = 10;
    //camera
    public var cameraX:Int = 0;
    public var cameraY:Int = 0;
    public var diffX:Int = 0;
    public var diffY:Int = 0;
    //text
    public var text:Text;
    //clean area
    var cleanX:Int = 0;
    var cleanY:Int = 0;

    //scale used for zoom in and out
    public var scale(get, set):Float;
    function get_scale():Float 
    {
        return scaleX;
    }
    function set_scale(scale:Float):Float 
    {
        scaleX = scale;
        scaleY = scale;
        center();
        return scale;
    }
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
        data = new GameData();

        #if openfl
        super();
        stage.color = 0xFFFFFF;
        ground = new Ground(this);
        objects = new Objects(this);
        //tile selector
        select = new Shape();
        select.cacheAsBitmap = true;
        select.graphics.lineStyle(2,0xB7B7B7);
        select.graphics.drawRect(0,0,Static.GRID,Static.GRID);

        draw = new Draw(this);
        dialog = new Dialog(this);
        addChild(ground);
        addChild(select);
        addChild(objects);
        Main.screen.addChild(draw);
        Main.screen.addChild(dialog);
        text = new Text();
        text.align = LEFT;
        text.cacheAsBitmap = false;
        Main.screen.addChild(text);
        bitmap = new Bitmap();
        addChild(bitmap);
        #end
        //connect
        if (true)
        {
            Main.client.login.accept = function()
            {
                trace("accept");
                //set message reader function to game
                Main.client.message = message;
                Main.client.end = end;
                Main.client.login = null;
                Main.client.tag = null;
                index = 0;
            }
            Main.client.login.reject = function()
            {
                trace("reject");
                Main.client.login = null;
            }
            //Main.client.login.email = "test@test.co.uk";
            //Main.client.login.key = "WC2TM-KZ2FP-LW5A5-LKGLP";
            Main.client.login.email = "test@test.com";
            Main.client.login.key = "9UYQ3-PQKCT-NGQXH-YB93E";
            Main.client.message = Main.client.login.message;
            Main.client.ip = "game.krypticmedia.co.uk";
            Main.client.port = 8007;
            Main.client.connect();
        }else{
            #if openfl
            //playground
            objects.size(32,30);
            //player
            setPlayer(cast(objects.add(19,true),Player));
            Player.main.instance = new PlayerInstance([]);
            Player.main.instance.move_speed = 3;
            data.playerMap.set(0,Player.main);
            //bush
            objects.add(30);
            //sheep
            objects.add(575).animate(2);
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
    public function center()
    {
        if (Player.main != null)
        {
            x = -(Player.main.x - 20) * scale + Main.setWidth/2;
            y = -(Player.main.y - 40) * scale + Main.setHeight/2;
        }
    }
    var xs:Int = 0;
    var ys:Int = 0;
    override function update()
    {
        super.update();
        draw.update();
        text.text = "num " + objects.numTiles;
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
                program.setupGoal = false;
                Player.main.step(xs,ys);
            }
        }
        //selector
        selectX = Math.floor((objects.mouseX - Static.GRID/2)/Static.GRID);
        selectY = Math.floor((objects.mouseY - Static.GRID/2)/Static.GRID);
        //set local for render
        select.x = objects.x + selectX * Static.GRID + Static.GRID/2;
        select.y = objects.y + selectY * Static.GRID + Static.GRID/2;
        //set x global
        selectX += -cameraX + 1;
        //flip
        selectY = (Static.tileHeight - selectY);
        //set y global
        selectY += -cameraY - 1;
        //players
        it = data.playerMap.iterator();
        while(it.hasNext())
        {
            it.next().update();
        }
    }
    public function sort()
    {
        objects.sortTiles(function (a:Tile,b:Tile):Int
        {
            if (a.y > b.y) return 1;
            return -1;
        });
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
        program.use(selectX,selectY);
    }
    override function mouseRightDown()
    {
        super.mouseRightDown();
        program.drop(selectX,selectY);
    }
    public function move(x:Float=0,y:Float=0)
    {
        //flip
        x *= -1;
        y *= -1;
        //move
        objects.x += x;
        objects.y += y;
        ground.x += x;
        ground.y += y;
    }
    override function mouseScroll(e:MouseEvent) 
    {
        super.mouseScroll(e);
        zoom(e.delta);
    }
    public function zoom(i:Int)
    {
        if (scale > 2 && i > 0 || scale < 0.2 && i < 0) return;
        scale += i * 0.08;
    }
    public function mapUpdate() 
    {
        trace("MAP UPDATE");
        //inital set camera
        if (inital)
        {
            cameraX = -data.map.x;
            cameraY = -data.map.y;
            diffX = -cameraX;
            diffY = -cameraY;
            inital = false;
            //width = 32, height = 30
            objects.size(mapInstance.width,mapInstance.height);
        }
        //clean before adding new
        clean();
        var obj:Object;
        for(j in mapInstance.y...mapInstance.y + mapInstance.height)
        {
            for (i in mapInstance.x...mapInstance.x + mapInstance.width)
            {
                //ground
                ground.add(data.map.biome.get(i,j),i,j);
                //floor
                objects.addFloor(data.map.floor.get(i,j),i,j);
                //objects
                objects.addObject(data.map.object.get(i,j),i,j);

            }
        }
        sort();
        //check for duplicates
        duplicate();
        ground.render();
    }
    public var list:Array<Tile> = [];
    public function clean()
    {
        var obj:Object = null;
        @:privateAccess list = objects.__group.__tiles.copy();
        for (i in 0...list.length)
        {
            obj = cast list.pop();
            if (obj.x < 0 || obj.x > objects.width || obj.y < 0 || obj.y > objects.height)
            {
                objects.removeTile(obj);
                obj = null;
            }
        }
        list = [];
    }
    public function duplicate()
    {
        var obj:Object = null;
        var obj2:Object = null;
        @:privateAccess list = objects.__group.__tiles.copy();
        for (i in 0...list.length)
        {
            obj = cast list.pop();
            for (i in 0...list.length)
            {
                obj2 = cast list[i];
                if (obj.tileX == obj2.tileX && obj.tileY == obj2.tileY)
                {
                    objects.removeTile(obj);
                    obj = null;
                    break;
                }
            }
        }
        list = [];
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
                //remaining of the tileset
                trace("fill " + objects.getFill());
                Player.main.sort();
                center();
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
            var tile:Object;
            if (change.speed > 0)
            {
                //move object 
            }else{
                for (i in 0...objects.numTiles)
                {
                    tile = cast objects.getTileAt(i);
                    if (tile.tileX == change.x && tile.tileY == change.y && tile.type != PLAYER)
                    {
                        objects.removeTile(tile);
                        break;
                    }
                }
                var obj = objects.add(change.floor != 0 ? change.floor : change.id);
                //set position todo
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
            dialog.say(input);
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