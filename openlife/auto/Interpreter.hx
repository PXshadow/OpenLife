package openlife.auto;

import openlife.data.object.ObjectData;
import haxe.ds.Vector;
@:expose("Interpreter")
class Interpreter
{
    var list:Vector<Int>;
    public function new(list:Vector<Int>)
    {
        this.list = list;
    }
    public function stringNumber(string:String):Int
    {
        return switch (string)
        {
            case "couple": 2;
            case "many": 6;
            case "few": 3;
            case "one": 1;
            case "two": 2;
            case "three": 3;
            case "four": 4;
            case "five": 5;
            case "six": 6;
            case "seven": 7;
            case "eight": 8;
            case "nine": 9;
            case "ten": 10;
            case "eleven": 11;
            case "twevle": 12;
            case "thirteen": 13;
            case "fourteen": 14;
            case "fiveteen": 15;
            case "cart" | "cartfull": 4;
            case "basket" | "basketfull": 3;
            default: 1;
        }
    }
    public function stringObject(words:Array<String>):Int
    {
        for (id in list)
        {
            var desc = new ObjectData(id,true).description.toUpperCase();
            var fail:Bool = false;
            for (word in words)
            {
                fail = desc.indexOf(word) == -1;
                if (fail) break;
            }
            if (!fail) return id;
        }
        return -1;
    }
    private inline function removePlural(string:String):String
    {
        if (string.substring(string.length - 1,string.length) == "s") string = string.substring(0,string.length - 1);
        return string;
    }
}