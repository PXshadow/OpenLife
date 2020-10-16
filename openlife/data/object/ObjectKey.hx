package openlife.data.object;
@:expose
@:enum abstract ObjectKey(Null<String>)
{
    public var CLOTHING = "clothing";
    public var FOOD = "food";
    public var TOOL = "tools";
    public var CONTAINER = "container";
    public var HEAT_SOURCE = "heat";
    public var WATER_SOURCE = "water";
    public var NATURAL = "natural";
    @:from private static function fromString(value:String):ObjectKey
    {
        return switch (value)
        {
            case "clothing": CLOTHING;
            case "food": FOOD;
            case "tools": TOOL;
            case "container": CONTAINER;
            case "heat": HEAT_SOURCE;
            case "water": WATER_SOURCE;
            case "natural": NATURAL;
            default: null;
        }
    }
}