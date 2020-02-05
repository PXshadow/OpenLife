import sys.FileSystem;
import haxe.io.Path;
import sys.io.File;
#if openfl
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
    public function new()
    {
        directory();
        super();
        /*var vector = Game.data.objectData();
        if (Game.data.nextObjectNumber > 0)
        {
            function complete()
            {
                trace("finish");
            }
            #if (target.threaded)
            sys.thread.Thread.create(() -> {
                objectData(vector);
                complete();
            });
            #else
            objectData(vector);
            complete();
            #end
        }
        cred();
        login();*/
        //connect();
        new data.sound.AiffData(File.getBytes(Game.dir + "sounds/1645.aiff"));
        
    }
    private function objectData(vector:Vector<Int>)
    {
        var int = Game.data.nextObjectNumber;
        var data:ObjectData;
        var dummyObject:ObjectData;
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
            //trace("id " + id);
        }
    }
    private function game()
    {
        objects = new Objects();
        ground = new Ground();
        addChild(objects);
        addChild(ground);
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
            //game();
            connect();
        }
        addChild(join);
    }
    override function mapChunk(instance:MapInstance) 
    {
        super.mapChunk(instance);
    }
}
#end

#if (!openfl)
import ImportAll;
class Main
{
    public static function main()
    {
        var client = new client.Client();
        client.accept = function()
        {
            trace("accept");
            client.message = message;
            client.accept = null;
        }
        client.reject = function()
        {
            trace("reject");
            client.reject = null;
        }
        client.message = client.login;
        client.connect();
        while (true)
        {
            client.update();
            Sys.sleep(0.2);
        }
    }
    public static function message(tag:client.ClientTag,input:Array<String>)
    {
        trace('$tag $input');
    }
}
#end