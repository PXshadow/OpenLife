package data.transition;
#if nativeGen @:nativeGen #end
class TransitionData
{
    public var lastUseActor:Bool = false;
    public var lastUseTarget:Bool = false;
    public var actorID:Int = 0;
    public var targetID:Int = 0;

    public var newActorID:Int = 0;
    public var newTargetID:Int = 0;
    public var autoDecaySeconds:Int = 0;
    public var actorMinUseFraction:Float = 0;
    public var targetMinUseFraction:Float = 0;
    public var reverseUseActor:Bool = false;
    public var reverseUseTarget:Bool = false;
    public var move:Int = 0;
    public var desireMoveDist:Bool = false;
    public var noUseActor:Bool = false;
    public var noUseTarget:Bool = false;

    public var playerActor:Bool = false;
    public var tool:Bool = false;
    public var targetRemains:Bool = false;
    public var decay:Int = 0;

    public function new(name:String,string:String)
    {
        //name
        var parts = name.split(".")[0].split("_");
        lastUseActor = (parts[2] == "LA");
        lastUseTarget = (parts[2] == "LT" || parts[2] == "L");
        actorID = Std.parseInt(parts[0]);
        targetID = Std.parseInt(parts[1]);
        //data
        var data = string.split(" ");
        //trace("data " + data);
        newActorID = Std.parseInt(data[0]);
        newTargetID = Std.parseInt(data[1]);
        autoDecaySeconds = Std.parseInt(data[2]);

        actorMinUseFraction = Std.parseFloat(data[3]);
        targetMinUseFraction = Std.parseFloat(data[4]);
        reverseUseActor = data[5] == "1";
        reverseUseTarget = data[6] == "1";
        move = Std.parseInt(data[7]);
        desireMoveDist = data[8] == "1";
        noUseActor = data[9] == "1";
        noUseTarget = data[10] == "1";

        playerActor = (actorID == 0);
        tool = (actorID >= 0 && actorID == newActorID);
        targetRemains = (targetID >= 0 && targetID == newTargetID);
    }
}