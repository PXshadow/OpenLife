package data;
#if openfl
import openfl.geom.Rectangle;
import haxe.io.BytesInput;
import lime.app.Future;
import haxe.io.Bytes;
import openfl.utils.ByteArray;
import haxe.io.Input;
import format.tga.*;

class TgaData
{
    //output
    public var bytes:ByteArray;
    public var rect:Rectangle;
    //to read
    private var r:Reader;
    private var d:Data;
    public function new()
    {

    }
    public function read(data:Bytes)
    {
        r = new Reader(new BytesInput(data,0,data.length));
        d = r.read();
        rect = new Rectangle(0,0,d.header.width,d.header.height);
        bytes = Tools.extract32(d,true);
    }
}
#end