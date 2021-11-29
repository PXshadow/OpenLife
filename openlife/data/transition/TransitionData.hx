package openlife.data.transition;

import openlife.settings.ServerSettings;
import openlife.data.object.ObjectData;
@:expose
class TransitionData
{
    public var addedBy = "Not Set"; // Arcurus gives a hint from who this Transiton was created, like a normal transition, or category or if it was patched

    //Last use determines whether the current transition is used when numUses is greater than 1
    public var lastUseActor:Bool = false;
    public var lastUseTarget:Bool = false;  // USED in transitions

    //actor + target = new actor + new target
    public var actorID:Int = 0; // USED in transitions
    public var targetID:Int = 0; // USED in transitions

    public var newActorID:Int = 0; // USED in transitions
    public var newTargetID:Int = 0; // USED in transitions

    public var autoDecaySeconds:Int = 0; // USED in time transitions

    //MinUse for variable-use objects that occasionally use more than one "use", this sets a minimum per interaction.
    public var actorMinUseFraction:Float = 0;
    public var targetMinUseFraction:Float = 0;

    public var reverseUseActor:Bool = false;
    public var reverseUseTarget:Bool = false; // USED in transitions
    public var move:Int = 0;
    public var desireMoveDist:Bool = false;
    public var noUseActor:Bool = false;
    public var noUseTarget:Bool = false;

    public var playerActor:Bool = false;
    public var tool:Bool = false;
    public var targetRemains:Bool = false;

    public static function createNewFromFile(fileName:String,string:String):TransitionData{

      var t = new TransitionData();

      t.parseFilename(fileName);
      t.parseData(string.split(" "));

      return t;
    }

    public function new(actorID:Int = 0, targetID:Int = 0, newActorID:Int = 0, newTargetID:Int = 0)
    {
        this.actorID = actorID;
        this.targetID = targetID;
        this.newActorID = newActorID;
        this.newTargetID = newTargetID;
    }

    public function clone():TransitionData
      {
        // Reflect seems to give back null :/
        //return Reflect.copy(this);
  
        var t = new TransitionData();
        var trans = this;

        t.lastUseActor = trans.lastUseActor;
        t.lastUseTarget = trans.lastUseTarget;
    
        t.actorID = trans.actorID;
        t.targetID = trans.targetID;
    
        t.newActorID = trans.newActorID;
        t.newTargetID = trans.newTargetID;
    
        t.autoDecaySeconds = trans.autoDecaySeconds;
    
        t.actorMinUseFraction = trans.actorMinUseFraction;
        t.targetMinUseFraction = trans.targetMinUseFraction;
    
        t.reverseUseActor = trans.reverseUseActor;
        t.reverseUseTarget = trans.reverseUseTarget;
        t.move = trans.move;
        t.desireMoveDist = trans.desireMoveDist;
        t.noUseActor = trans.noUseActor;
        t.noUseTarget = trans.noUseTarget;
    
        t.playerActor = trans.playerActor;
        t.tool = trans.tool;
        t.targetRemains = trans.targetRemains;

        return t;
    }

    private function parseFilename(fileName:String)
    {
        //name
        var parts = fileName.split(".")[0].split("_");
        lastUseActor = (parts[2] == "LA");
        lastUseTarget = (parts[2] == "LT" || parts[2] == "L");
        actorID = Std.parseInt(parts[0]);
        targetID = Std.parseInt(parts[1]);
    }

    private function parseData(data:Array<String>)
    {
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

    public function isGeneric()
    {
      return targetID == -1 && newTargetID == 0 && actorID != newActorID;
    }

    public function isAttack():Bool
    {
      return targetID == 0 && !lastUseActor && !lastUseTarget;
    }

    public function isLastUse():Bool
    {
      return lastUseActor || lastUseTarget;
    }

    public function targetsPlayer()
    {
      return targetID == 0 || targetID == -1 && new ObjectData(actorID).foodValue > 0;
    }

    public function calculateDecay(seconds:Int):String
    {
        if (seconds < 0)
          return '${-seconds} hours';
        if (seconds > 0 && seconds % 60 == 0)
          return '${seconds/60} minutes';
        if (seconds > 0)
          return '$seconds seconds';
        return "";
    }
    
    public function toString():String
    {
      var s = addedBy;
      s += ' $actorID + $targetID = $newActorID + $newTargetID ';
      
      s += 'lastUseActor: $lastUseActor ';
      s += 'lastUseTarget: $lastUseTarget ';

      s += 'autoDecaySeconds: $autoDecaySeconds ';

      //TODO ??? MinUse for variable-use objects that occasionally use more than one "use", this sets a minimum per interaction.
      //public var actorMinUseFraction:Float = 0;
      //public var targetMinUseFraction:Float = 0;

      s += 'reverseUseActor: $reverseUseActor ';
      s += 'reverseUseTarget: $reverseUseTarget ';
      s += 'move: $move ';

      //public var desireMoveDist:Bool = false;
      s += 'noUseActor: $noUseActor ';
      s += 'noUseTarget: $noUseTarget ';

      s += 'playerActor: $playerActor ';
      //public var tool:Bool = false;
      s += 'targetRemains: $targetRemains ';

      return s;
    }

    //public function traceTransition(s:String = "", targetDescContains:String = "")
    public function traceTransition(s:String = "", forceTrace = false, smalTrace = false)
    {
      var transition = this;

      var objectDataActor = ObjectData.getObjectData(transition.actorID);
      var objectDataTarget = ObjectData.getObjectData(transition.targetID);
      var objectDataNewActor = ObjectData.getObjectData(transition.newActorID);
      var objectDataNewTarget = ObjectData.getObjectData(transition.newTargetID);

      var actorDescription = "";
      var targetDescription = "";
      var newActorDescription = "";
      var newTargetDescription = "";

      if(objectDataActor != null) actorDescription = objectDataActor.description;
      if(objectDataTarget != null) targetDescription = objectDataTarget.description;
      if(objectDataNewActor != null) newActorDescription = objectDataNewActor.description;
      if(objectDataNewTarget != null) newTargetDescription = objectDataNewTarget.description;

      var dontTraceActor = actorDescription.indexOf(ServerSettings.traceTransitionByActorDescription) == -1;
      var dontTraceTarget = targetDescription.indexOf(ServerSettings.traceTransitionByTargetDescription) == -1 ;

      if(forceTrace == false && dontTraceActor && dontTraceTarget && transition.actorID != ServerSettings.traceTransitionByActorId && transition.targetID != ServerSettings.traceTransitionByTargetId) return;

      var description = getDesciption(smalTrace);
      
      trace('$s $description');
  }

  public function getDesciption(smal:Bool = true) : String
  {
      var transition = this;

      var objectDataActor = ObjectData.getObjectData(transition.actorID);
      var objectDataTarget = ObjectData.getObjectData(transition.targetID);
      var objectDataNewActor = ObjectData.getObjectData(transition.newActorID);
      var objectDataNewTarget = ObjectData.getObjectData(transition.newTargetID);

      var actorDescription = "";
      var targetDescription = "";
      var newActorDescription = "";
      var newTargetDescription = "";

      if(objectDataActor != null) actorDescription = objectDataActor.description;
      if(objectDataTarget != null) targetDescription = objectDataTarget.description;
      if(objectDataNewActor != null) newActorDescription = objectDataNewActor.description;
      if(objectDataNewTarget != null) newTargetDescription = objectDataNewTarget.description;

      if(smal) return '$actorID + $targetID = $newActorID + $newTargetID / $actorDescription + $targetDescription  -->  $newActorDescription + $newTargetDescription\n';
      else return '$transition $actorDescription + $targetDescription  -->  $newActorDescription + $newTargetDescription\n';
  }
}