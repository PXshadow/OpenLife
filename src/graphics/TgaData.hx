package graphics;
import haxe.ds.Vector;
import haxe.io.Output;
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
    public var data:Data;
    public function new()
    {

    }
    public function read(bytes:Bytes)
    {
        r = new Reader(new BytesInput(bytes,0,bytes.length));
        this.data = r.read();
        
        rect = new Rectangle(0,0,data.header.width,data.header.height);
        this.bytes = ByteArray.fromBytes(Tools.extract32(data,true));
    }
    public function write(data:Data,output:Output)
    {
        var w:Writer = new Writer(output);
        w.write(data);
        output.close();
    }
    public function bytesToVector(bytes:Bytes):Vector<Int>
    {
        var imageData:Vector<Int> = new Vector<Int>(Std.int(bytes.length/4));
        for (i in 0...imageData.length)
        {
            imageData[i] = bytes.getInt32(i);
        }
        return imageData;
    }
}
#end