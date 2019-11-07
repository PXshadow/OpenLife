package debugger;
import lime.system.System;
//print out a data class in order to quickly use for a demo project
class ClassPrintOut
{
    var string:String = "";
    public function new (c:Class<Dynamic>)
    {
        a("package;");
        a("class " + Type.getClassName(c));
        a("{");
        
        //a(" var ")
        trace("fields " + Type.getClassFields(c));
    }
    private function a(sub:String)
    {
        string += sub + "\n";
    }
}