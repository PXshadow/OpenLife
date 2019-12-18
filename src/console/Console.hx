package console;
#if openfl
import openfl.ui.Keyboard;
import openfl.net.URLRequest;
import openfl.events.TextEvent;
import openfl.display.Shape;
import openfl.display.DisplayObjectContainer;
import ui.Text;
#end

class Console #if openfl extends DisplayObjectContainer #end
{
    var length:Int = 0;
    #if hscript
    var parser:hscript.Parser;
    var interp:Interp;
    #end
    var history:Array<String> = [];
    var command:Command;
    //debug elements
    public var debug:Bool = false;
    #if openfl
    public var input:Text;
    var output:Text;
    var shape:Shape;
    #end
    public function new()
    {
        #if openfl
        super();
        visible = false;
        shape = new Shape();
        shape.cacheAsBitmap = true;
        addChild(shape);

        input = new Text("",LEFT,20,0xFFFFFF);
        length = input.length;
        input.selectable = true;
        input.mouseEnabled = true;
        input.cacheAsBitmap = false;
        input.type = INPUT;
        input.multiline = false;
        input.restrict = "^`";
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
        #end
        //hscript
        #if hscript
        parser = new Parser();
        parser.allowJSON = true;
        parser.allowJSON = true;
        interp = new Interp();
        command = new Command();
        //interp variables default
        set("math",Math);
        set("grid",Static.GRID);
        set("util",console.Util);
        #end
    }
    public function set(name:String,value:Dynamic)
    {
        #if hscript
        interp.variables.set(name,value);
        #end
    }
    public function print(inp:String,out:String)
    {
        #if openfl
        if(output.numLines > 9)
        {
            output.text = "";
        }
        output.appendText(">" + inp + "\n" + out + "\n");
        #else
        trace(out);
        #end
    }
    #if openfl
    //client events
    public function resize(width:Float)
    {
        //graphics
        shape.graphics.clear();
        shape.graphics.beginFill(0,0.85);
        shape.graphics.drawRect(0,0,width,296);
        shape.graphics.endFill();
        shape.graphics.lineStyle(1,0xFFFFFF);
        shape.graphics.moveTo(0,260);
        shape.graphics.lineTo(width,260);
        //text widths
        input.width = width;
        output.width = width;
    }
    public function keyDown(code:Int):Bool
    {
        switch(code)
        {
            case Keyboard.TAB | Keyboard.BACKQUOTE:
            //toggle vis
            visible = !visible;
            if(visible)
            {
                input.setSelection(input.length,input.length);
            }else{
                stage.focus = null;
            }
        }
        if (stage.focus != input) return false;
        //input needs to be focused on for keys below
        switch(code)
        {
            case Keyboard.DOWN:
            //fast delete line
            input.text = "";
            case Keyboard.UP:
            //pull up history
            previous();
            case Keyboard.ENTER:
            enter();
        }
        return true;
    }
    public function update()
    {
        if(stage.focus == input)
        {
            if(input.length != length)
            {
                if(input.length > length)
                {
                    
                }else{
                    //subtract
                }
                length = input.length;
            }
        }
    }
    #end
    public function previous()
    {
        #if openfl
        if (history.length > 0) 
        {
            input.text = history.pop();
            input.setSelection(input.length,input.length);
        }
        #end
    }
    #if openfl
    public function enter()
    {
        if(input.length == 0) return;
        var text = input.text;
        //coammnd outside of hscript
        if(command.run(text)) return;
        input.text = "";
        //multiline reset
        if(output.numLines > 9)
        {
            output.text = "";
        }
        run(text);
    }
    #end
    public function run(text:String)
    {
        //set history
        history.push(text);
        //attempt hscript
        try {
            #if hscript
            print(text,interp.expr(parser.parseString(text)));
            #end
        }catch(e:Dynamic)
        {
            print(text,e);
        }
    }
    private function getFields(Object:Dynamic):Array<String>
	{
		var fields = [];
		if (Std.is(Object, Class)) // passed a class -> get static fields
			fields = Type.getClassFields(Object);
		else if (Std.is(Object, Enum))
			fields = Type.getEnumConstructs(Object);
		else if (Reflect.isObject(Object)) // get instance fields
			fields = Type.getInstanceFields(Type.getClass(Object));

		// on Flash, enums are classes, so Std.is(_, Enum) fails
		fields.remove("__constructs__");

		var filteredFields = [];
		for (field in fields)
		{
			// don't add property getters / setters
			if (StringTools.startsWith(field,"get_") || StringTools.startsWith(field,"set_"))
			{
				var name = field.substr(4);
				// property without a backing field, needs to be added
				//if (!fields.contains(name) && !filteredFields.contains(name))
                if (fields.indexOf(name) == -1 && filteredFields.indexOf(name) == -1)
					filteredFields.push(name);
			}
			else
				filteredFields.push(field);
		}

		return filteredFields;
	}
}
#if hscript
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
#end