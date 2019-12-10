package scripts;
#if openfl
import sys.io.File;
import openfl.display.PNGEncoderOptions;
import data.TgaData;
import sys.FileSystem;
import openfl.display.BitmapData;
import haxe.io.Path;

class ImageGenerate
{
    var reader:TgaData = new TgaData();
    public function new(folder:String)
    {
        folder += "/";
        if (!FileSystem.exists(Game.dir + folder) || !FileSystem.isDirectory(Game.dir + folder)) return;
        var bmd:BitmapData;
        for (dir in FileSystem.readDirectory(Game.dir + folder))
        {
            if (Path.extension(dir) != "tga") continue;
            reader.read(File.getBytes(Game.dir + folder + dir));
            //generate new bitmapdata
            bmd = new BitmapData(Std.int(reader.rect.width),Std.int(reader.rect.height));
            //fill
            bmd.setPixels(reader.rect,reader.bytes);
            //save to png
            File.saveBytes(Game.dir + folder + Path.withoutExtension(dir) + ".png",bmd.encode(reader.rect,new PNGEncoderOptions(false)));
        }
    }
}
#end