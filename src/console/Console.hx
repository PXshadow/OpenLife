package console;
import openfl.ui.Keyboard;
import openfl.net.URLRequest;
import hscript.Parser;
import openfl.events.TextEvent;
import openfl.display.Shape;
import openfl.display.DisplayObjectContainer;
import ui.Text;

class Console extends DisplayObjectContainer
{
    public var input:Text;
    var output:Text;
    var shape:Shape;
    var length:Int = 0;
    var parser:Parser;
    public static var interp:Interp;
    var history:Array<String> = [];
    var command:Command;
    public function new()
    {
        super();
        visible = false;
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
        output = new Text("",LEFT,18,0xFFFFFF);
        output.cacheAsBitmap = false;
        //output.selectable = true;
        //output.mouseEnabled = true;
        output.mouseWheelEnabled = true;
        output.tabEnabled = false;
        output.height = 260 - 6;
        addChild(output);
        //hscript
        parser = new Parser();
        parser.allowJSON = true;
        parser.allowJSON = true;
        interp = new Interp();
        //variables
        interp.variables.set("Math",Math);
        interp.variables.set("Std",Std);
        interp.variables.set("client",Main.client);
        interp.variables.set("width",Main.setWidth);
        interp.variables.set("height",Main.setHeight);
        interp.variables.set("grid",Static.GRID);
        //utils
        interp.variables.set("util",console.Util);
        interp.variables.set("Util",console.Util);

        command = new Command(this);
    }
    public function print(inp:String,out:String)
    {
        if(output.numLines > 9)
        {
            output.text = "";
        }
        output.appendText(">" + inp + "\n" + out + "\n");
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
    public function keyDown(code:Int)
    {
        switch(code)
        {
            case Keyboard.TAB:
            //toggle vis
            visible = !visible;
            if(visible)
            {
                input.setSelection(input.length,input.length);
            }else{
                stage.focus = null;
            }
        }
        if (stage.focus != input) return;
        //input needs to be focused on for keys below
        switch(code)
        {
            case Keyboard.DOWN:
            //fast delete line
            input.text = ">";
            case Keyboard.UP:
            //pull up history
            previous();
            case Keyboard.ENTER:
            enter();
        }
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
        if (history.length > 0) 
        {
            input.text = ">" + history.pop();
            input.setSelection(input.length,input.length);
        }
        
    }
    public function enter()
    {
        if(input.length == 1) return;
        var text = input.text.substring(1,input.length);
        //coammnd outside of hscript
        if(command.run(text)) return;
        input.text = "";
        //multiline reset
        if(output.numLines > 9)
        {
            output.text = "";
        }
        //set history
        history.push(text);
        //attempt hscript
        try {
            print(text,interp.expr(parser.parseString(text)));
        }catch(e:Dynamic)
        {
            print(text,e);
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
