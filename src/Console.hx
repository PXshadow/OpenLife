import hscript.Parser;
import openfl.events.TextEvent;
import openfl.display.Shape;
import openfl.display.DisplayObjectContainer;

class Console extends DisplayObjectContainer
{
    var input:Text;
    var output:Text;
    var shape:Shape;
    var length:Int = 0;
    var parser:Parser;
    var interp:Interp;
    var history:Array<String> = [];
    public function new()
    {
        super();
        shape = new Shape();
        shape.cacheAsBitmap = true;
        addChild(shape);

        input = new Text(">",LEFT,20,0xFFFFFF);
        //input.restrict = "^_";
        length = input.length;
        input.selectable = true;
        input.mouseEnabled = true;
        input.cacheAsBitmap = false;
        input.type = INPUT;
        input.multiline = false;
        input.y = 260 + 2;
        addChild(input);
        //output
        output = new Text("...",LEFT,18,0xFFFFFF);
        //output.selectable = true;
        //output.mouseEnabled = true;
        output.tabEnabled = false;
        output.height = 260;
        addChild(output);
        //hscript
        parser = new Parser();
        parser.allowJSON = true;
        parser.allowJSON = true;
        interp = new Interp();
        //variables
        interp.variables.set("Math",Math);
        interp.variables.set("Std",Std);
        interp.variables.set("player",Player.main);
    }
    public function resize(width:Float)
    {
        //graphics
        shape.graphics.clear();
        shape.graphics.beginFill(0);
        shape.graphics.drawRect(0,0,width,296);
        shape.graphics.endFill();
        shape.graphics.lineStyle(1,0xFFFFFF);
        shape.graphics.moveTo(0,260);
        shape.graphics.lineTo(width,260);
        //text widths
        input.width = width;
        output.width = width;
    }
    public function update()
    {
        if(input.selectable)
        {
            if(input.length != length)
            {
                length = input.length;
                if(input.length > length)
                {
                    //add

                }else{
                    //subtract
                    if (length == 0) 
                    {
                        input.appendText(">");
                        input.setSelection(1,1);
                    }
                }
            }
        }
    }
    public function previous()
    {
        input.text = ">" + history.pop();
    }
    public function enter()
    {
        var text = input.text.substring(1,input.length);
        input.text = "";
        output.appendText(">" + text + "\n");
        history.push(text);
        try {
            output.appendText(interp.expr(parser.parseString(text)) + "\n");
        }catch(e:Dynamic)
        {
            output.appendText(e + "\n");
        }
    }
}
private class Interp extends hscript.Interp
{
	public function getGlobals():Array<String>
	{
		return toArray(locals.keys()).concat(toArray(variables.keys()));
	}

	function toArray<T>(iterator:Iterator<T>):Array<T>
	{
		var array = [];
		for (element in iterator)
			array.push(element);
		return array;
	}

	override function get(o:Dynamic, f:String):Dynamic
	{
		if (o == null)
			error(EInvalidAccess(f));
		return Reflect.getProperty(o, f);
	}

	override function set(o:Dynamic, f:String, v:Dynamic):Dynamic
	{
		if (o == null)
			error(EInvalidAccess(f));
		Reflect.setProperty(o, f, v);
		return v;
	}
}
