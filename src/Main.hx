import game.Player;
import data.object.player.PlayerInstance;
import sys.FileSystem;
import haxe.io.Path;
import sys.io.File;
#if openfl
import openfl.events.MouseEvent;
import openfl.events.KeyboardEvent;
import openfl.ui.Keyboard;
import openfl.display.Shape;
import openfl.events.Event;
import openfl.display.Tile;
import lime.media.AudioSource;
import openfl.media.SoundChannel;
import openfl.media.Sound;
import haxe.ds.Vector;
import lime.app.Future;
import data.object.ObjectData;
import game.Game;
import game.Ground;
import game.Objects;
import data.map.MapInstance;
import ui.Text;
import ui.InputText;
import ui.Button;
class Main extends game.Game
{
    var objects:Objects;
    var ground:Ground;
    var player:Player;
    public function new()
    {
        directory();
        super();
        var vector = Game.data.objectData();
        if (Game.data.nextObjectNumber > 0)
        {
            objectData(vector);
        }
        cred();
        //login();
        game();
        connect();
        stage.addEventListener(Event.RESIZE,resize);
        stage.addEventListener(KeyboardEvent.KEY_DOWN,keyDown);
        stage.addEventListener(MouseEvent.MOUSE_DOWN,mouseDown);
        stage.addEventListener(MouseEvent.MOUSE_WHEEL,mouseWheel);
        stage.addEventListener(Event.ENTER_FRAME,update);
        //new data.sound.AiffData(File.getBytes(Game.dir + "sounds/1645.aiff"));
        //resize(null);
    }
    private function mouseWheel(e:MouseEvent)
    {
        zoom(e.delta);
    }
    private function mouseDown(_)
    {

    }
    private function keyDown(e:KeyboardEvent)
    {
        switch (e.keyCode)
        {
            case Keyboard.I: zoom(1);
            case Keyboard.O: zoom(-1);
        }
    }
    private function zoom(i:Int)
    {
        if (objects.scale > 2 && i > 0 || objects.scale < 0.2 && i < 0) return;
        objects.scale += i * 0.08;
    }
    private function resize(_)
    {
        if (objects != null)
        {
            objects.width = stage.stageWidth;
            objects.height = stage.stageHeight;
        }
    }
    private function objectData(vector:Vector<Int>)
    {
        var int = Game.data.nextObjectNumber;
        var data:ObjectData;
        var dummyObject:ObjectData;
        var i:Int = 0;
        for (id in vector)
        {
            data = new ObjectData(id);
            if (data.numUses > 1)
            {
                for (j in 1...data.numUses - 1)
                {
                    dummyObject = data.clone();
                    dummyObject.id = ++int;
                    dummyObject.numUses = 0;
                    dummyObject.dummy = true;
                    dummyObject.dummyParent = data.id;
                    Game.data.objectMap.set(dummyObject.id,dummyObject);
                }
            }
            Game.data.objectMap.set(data.id,data);
            if (i++ % 200 == 0) trace("i " + i);
        }
    }
    private function game()
    {
        trace("create game");
        objects = new Objects();
        ground = new Ground();
        addChild(ground);
        addChild(objects);
    }
    private function login()
    {
        var keyText = new Text("Key",LEFT,24,0xFFFFFF);
        keyText.y = 100;
        var emailText = new Text("Email",LEFT,24,0xFFFFFF);
        emailText.y = 50;
        addChild(keyText);
        addChild(emailText);
        
        var serverText = new Text("Address",LEFT,24,0xFFFFFF);
        var portText = new Text("Port",LEFT,24,0xFFFFFF);
        serverText.y = 150;
        portText.y = 200;
        addChild(serverText);
        addChild(portText);

        var keyInput = new InputText();
        keyInput.x = 100;
        keyInput.y = 100;
        addChild(keyInput);
        var emailInput = new InputText();
        emailInput.x = 100;
        emailInput.y = 50;
        addChild(emailInput);

        var serverInput = new InputText();
        serverInput.x = 100;
        serverInput.y = 150;
        addChild(serverInput);

        var portInput = new InputText();
        portInput.x = 100;
        portInput.y = 200;
        addChild(portInput);
        //fill
        /*if (!settings.fail)
        {
            emailInput.text = settings.data.get("email");
            keyInput.text = settings.data.get("accountKey");
            if (valid(settings.data.get("customServerAddress"))) serverInput.text = string;
            if (valid(settings.data.get("customServerPort"))) portInput.text = string;
        }*/
        var join = new Button();
        join.text = "Join";
        join.y = 250;
        join.graphics.beginFill(0x808080);
        join.graphics.drawRect(0,0,60,30);
        join.Click = function(_)
        {
            if (emailInput.text.indexOf("@") == -1 || 
            emailInput.text.length < 5 || 
            keyInput.text.length < 4 || 
            serverInput.text.indexOf(".") == -1 ||
            serverInput.text.length < 4 ||
            Std.parseInt(portInput.text) == null
            ) return;
            //settings set
            settings.data.set("email",emailInput.text);
            settings.data.set("accountKey",keyInput.text);
            settings.data.set("customServerAddress",serverInput.text);
            settings.data.set("customServerPort",portInput.text);
            //client set
            client.ip = settings.data.get("customServerAddress");
            client.port = Std.parseInt(settings.data.get("customServerPort"));
            client.email = settings.data.get("email");
            client.key = settings.data.get("accountKey");
            //remove login
            removeChild(keyText);
            removeChild(emailText);
            removeChild(serverText);
            removeChild(portText);
            removeChild(keyInput);
            removeChild(emailInput);
            removeChild(serverInput);
            removeChild(portInput);
            removeChild(join);
            keyText = emailText = serverText = portText = null;
            keyInput = emailInput = serverInput = portInput = null;
            join = null;
            //start game
            game();
            connect();
        }
        addChild(join);
    }
    private function update(_) 
    {
        client.update();
        if (ground != null)
        {
            ground.x = objects.group.x;
            ground.y = objects.group.y;
            ground.scaleX = objects.group.scaleX;
            ground.scaleY = objects.group.scaleY;
        }
    }
    override function playerUpdate(instances:Array<PlayerInstance>) 
    {
        super.playerUpdate(instances);
        for (i in 0...instances.length)
        {
            objects.addPlayer(instances[i]);
            objects.player.x = 0;
            objects.player.y = 0;
        }
        objects.addPlayer(instances.pop());
        player = objects.player;
    }
    override function mapChunk(instance:MapInstance) 
    {
        super.mapChunk(instance);
        trace("map " + instance.toString());
        for (i in instance.x...instance.x + instance.width)
        {
            for (j in instance.y...instance.y + instance.height)
            {
                ground.add(Game.data.map.biome.get(i,j),i,j);
                objects.add([Game.data.map.floor.get(i,j)],i,j);
            }
        }
        for (i in instance.x...instance.x + instance.width)
        {
            for (j in instance.y...instance.y + instance.height)
            {
                objects.add(Game.data.map.object.get(i,j),i,j);
            }
        }
        Game.data.map.chunks.push(instance);
        ground.render();
    }
}
#end

#if (!openfl)
import ImportAll;
class Main extends game.Game
{
    public static function main()
    {
        new Main();
    }
    public function new()
    {
        directory();
        super();
        cred();
        client.ip = "localhost";
        client.port = 8005;
        connect();
        while (true)
        {
            client.update();
            Sys.sleep(0.2);
        }
    }
    override function playerUpdate(instances:Array<PlayerInstance>) {
        super.playerUpdate(instances);
        var str = "";
        for (instance in instances)
        {
            str += instance.toString() + "\n";
        }
        trace("player update: " + str);
    }
}
#end