package states.serverlist;

import openfl.text.TextFormat;
import haxe.Timer;
import lime.app.Future;
import openfl.events.MouseEvent;
import openfl.display.Sprite;
import haxe.Http;
class ServerList extends Sprite
{
    public var list:Array<ServerType> = [];
    public var total:{current:Int,max:Int} = null;
    private var complete:Bool = false;
    private var host:Text;
    private var users:Text;
    public var ip:String;
    public var port:Int;
    @:isVar private var select(get,set):Int = 0;
    function set_select(value:Int):Int
    {
        select = value;
        if(select < host.length && select >= 0)
        {
            var index = host.getLineOffset(select);
            host.setTextFormat(new TextFormat(host.defaultTextFormat.font,
            host.defaultTextFormat.size,0xFF0000),index,index + host.getLineLength(select));
        }
        return select;
    }
    function get_select():Int
    {
        return select;
    }
    public function new()
    {
        super();
        buttonMode = true;
        addText();
        pull();
        var t = new Timer(10 * 1000);
        t.run = function()
        {
            if(complete) pull();
        }
        addEventListener(MouseEvent.CLICK,click);
    }
    private function addText()
    {
        host = new Text("Host");
        host.color = 0xFFFFFF;
        host.height = 600;
        host.width = 180;
        host.spacing = 4;
        users = new Text("info");
        users.color = 0xFFFFFF;
        users.x = host.width + 10;
        users.spacing = 4;
        users.align = RIGHT;
        users.width = 80;
        users.height = 600;
        addChild(host);
        addChild(users);
    }
    private  function pull()
    {
        //get data
        var http = new Http("http://onehouronelife.com/reflector/server.php?action=report");
        http.onData = function(data:String)
        {
            //async
            //var f = new Future(function()
            //{
            data = Data.removeInstanceString(data,"Remote servers");
            data = Data.removeInstanceString(data,"-->");
            data = Data.removeInstanceString(data,"<br>");
            data = Data.removeInstanceString(data,":");
            data = Data.removeInstanceString(data,"/");
            data = Data.removeInstanceString(data,"|");
            data = Data.removeInstanceString(data,"-");
            data = StringTools.replace(data,"  "," ");
            //get total data
            var tInt = data.indexOf("Total");
            var tArray:Array<String> = data.substring(tInt + 6 + 1,data.length).split(" ");
            total = {current: Std.parseInt(tArray[0]),max: Std.parseInt(tArray[1])};
            //remove total
            data = data.substring(0,tInt);
            //parse data
            list = [];
            var count:Int = 0;
            var i:Int = listNew();
            for(string in data.split(" "))
            {
                if(string == "")continue;
                switch(count)
                {
                    case 0:
                    list[i].ip = string;
                    case 1:
                    list[i].port = Std.parseInt(string);
                    case 2:
                    list[i].current = Std.parseInt(string);
                    case 3:
                    list[i].max = Std.parseInt(string);
                }
                //counter
                count ++;
                if(count >= 4)
                {
                    count = 0;
                    i = listNew();
                }
            }
            list.pop();
            //},true);
            //f.onComplete(function(_)
            //{
                complete = true;
                display();
            //});
            http = null;
        }
        new Future(function()
        {
            http.request(false);
        },true);
    }
    private function click(_)
    {
        host.invalidate();
        select = Std.int(mouseY/(12 + 4 + 1));
        if(list.length > 0 && select >= 0) 
        {
            ip = list[select].ip;
            port = list[select].port;
        }
    }
    private function display()
    {
        //fill text
        host.text = "";
        users.text = "";
        for(item in list)
        {
            host.appendText(item.ip + "\n");
            users.appendText(item.current + "/" + item.max + "\n");
        }
        //set format again
        select = select;
        //underneath
        graphics.beginFill(0,0);
        graphics.drawRect(0,0,width,height);
        //animate
        //motion.Actuate.tween(this,0.2,{alpha:0.5}).reflect(true).repeat(1).ease(Elastic.easeInOut);
    }
    public function listNew():Int
    {
        return list.push({ip: "",port: 0,current: 0,max: 0}) - 1;
    }
}
typedef ServerType = {ip:String,port:Int,current:Int,max:Int};