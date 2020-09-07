package openlife.auto;

import openlife.data.Pos;
import haxe.ds.Vector;
import openlife.engine.Program;


/**
 * nick name auto, will be a powerful class that uses program, and transition data to do automatic tasks.
 */
class Automation
{
    var program:Program;
    var list:Vector<Int>;
    public var interp:Interpreter;
    public function new(program:Program,list:Vector<Int>=null)
    {
        this.program = program;
        this.list = list;
        interp = new Interpreter(list);
    }
}
typedef Auto = Automation; 