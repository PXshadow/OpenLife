package openlife.data.transition;
@:expose("Category")
class Category
{
    public var ids:Array<Int>;
    public var weights:Array<Float>;
    public var parentID:Int = 0;
    public var pattern:Bool = false;
    public var probSet:Bool = false;
    public function new(text:String)
    {
        ids = [];
        weights = [];
        var lines = text.split("\n");
        var headers:Bool = true;
        for (line in lines)
        {
            headers ? headers = processHeader(line) : processObject(line);
        }
    }
    private function processHeader(line:String)
    {
        var parts = line.split("=");
        switch (StringTools.replace(parts[0],"\r",""))
        {
            case "parentID": parentID = Std.parseInt(parts[1]);
            case "pattern": pattern = true;
            case "probSet": probSet = true;
            case "numObjects": return false;
            default: throw 'Unknown category header |${parts[0]}|';
        }
        return true;
    }
    private function processObject(line:String)
    {
        var parts = line.split(" ");
        ids.push(Std.parseInt(parts[0]));
        if (probSet) weights.push(Std.parseFloat(parts[1]));
    }
    public function toString():String
    {
        return 'parent id: $parentID is pattern: $pattern prob: $probSet ids: $ids';
    }
}