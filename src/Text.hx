package;
import openfl.text.Font;
import openfl.Assets;
import haxe.io.Path;
import openfl.text.TextFormatAlign;
import openfl.text.TextFormat;
import openfl.text.TextField;

class Text extends TextField
{
    @:isVar public var align(null,set):TextFormatAlign;
    @:isVar public var color(null,set):UInt;
    @:isVar public var size(null,set):Int;
    @:isVar public var font(null,set):String = "_sans";
    @:isVar public var spacing(null,set):Int = 0;
    @:isVar public var leftMargin(null,set):Int = 0;
    @:isVar public var rightMargin(null,set):Int = 0;
    function set_leftMargin(value:Int):Int
    {
        leftMargin = value;
        updateFormat();
        return leftMargin;
    }
    function set_rightMargin(value:Int):Int
    {
        rightMargin = value;
        updateFormat();
        return rightMargin;
    }
    function set_spacing(value:Int):Int
    {
        spacing = value;
        updateFormat();
        return spacing;
    }
    function set_font(value:String):String
    {
        if(Path.extension(value) == "")
        {
            font = value;
            updateFormat();
        }else{
            Assets.loadFont(value).onComplete(function(f:Font)
            {
                font = f.fontName;
                updateFormat();
            });
        }
        return font;
    }
    function set_size(value:Int):Int
    {
        size = value;
        updateFormat();
        return size;
    }
    function set_color(value:UInt):UInt
    {
        color = value;
        updateFormat();
        return color;
    }
    function set_align(value:TextFormatAlign):TextFormatAlign
    {
        align = value;
        updateFormat();
        return align;
    }
    public function new(text:String="",align:TextFormatAlign=LEFT,size:Int=12,color:UInt=0,width:Int=100)
    {
        super();
        this.text = text;
        this.align = align;
        this.size = size;
        this.color = color;
        this.width = width;
        cacheAsBitmap = true;
        selectable = false;
        mouseEnabled = false;
        multiline = true;
        updateFormat();
    }
    private function updateFormat()
    {
        var format = new TextFormat(font,size,color,null,null,null,null,null,align,null,null,null,spacing);
        defaultTextFormat = format;
    }
    override function invalidate() 
    {
        super.invalidate();
        var format = new TextFormat(font,size,color,null,null,null,null,null,align,null,null,null,spacing);
        setTextFormat(format,0,text.length);
    }
}