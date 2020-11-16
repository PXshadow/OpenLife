package openlife.data.transition;

import openlife.server.Server;
import openlife.data.object.ObjectData;
import haxe.io.Path;
import sys.io.File;
import openlife.engine.Engine;
@:expose
class TransitionImporter
{
    public var transitions:Array<TransitionData>;
    public var categories:Array<Category>;

    private var transitionsByActorIdTargetId:Map<Int, Map<Int, TransitionData>>;

    public function new()
    {

    }
    public function importCategories()
    {
        categories = [];
        for (name in sys.FileSystem.readDirectory(Engine.dir + "categories"))
        {
            var category = new Category(File.getContent(Engine.dir + 'categories/$name'));
            categories.push(category);
        }
    }
    public function importTransitions()
    {        
        transitions = [];
        transitionsByActorIdTargetId = [];

        for (name in sys.FileSystem.readDirectory(Engine.dir + "transitions"))
        {
            //Last use determines whether the current transition is used when numUses is greater than 1
            //MinUse for variable-use objects that occasionally use more than one "use", this sets a minimum per interaction.
            var transition = new TransitionData(Path.withoutExtension(name),File.getContent(Engine.dir + 'transitions/$name'));
            //actor + target = new actor + new target
            transitions.push(transition);

            var transitionsByTargetId = transitionsByActorIdTargetId[transition.actorID];
            
            if(transitionsByTargetId == null){
                transitionsByTargetId = [];
                transitionsByActorIdTargetId[transition.actorID] = transitionsByTargetId;
            }

            var trans = transitionsByTargetId[transition.targetID];

            
            var objectDataActor = Server.objectDataMap[transition.actorID];
            var objectDataTarget = Server.objectDataMap[transition.targetID];

            var actorDescription = "";
            var targetDescription = "";

            if(objectDataActor != null) actorDescription = objectDataActor.description; //trace('actor: ${objectDataActor.description}');
            if(objectDataTarget != null) targetDescription = objectDataTarget.description;//trace('target: ${objectDataTarget.description}');
           
            //trace('New transition: a: ${transition.actorID} t: ${transition.targetID} - $actorDescription - $targetDescription');


            // if there is a transition allready, then there is an additional "last" transition
            if(trans != null){
                
                // TODO make map for last transitions
                //trace('Double transition: actor: ${trans.actorID} target: ${trans.targetID}');
            }

            transitionsByTargetId[transition.targetID] = transition;

            //trans.push(transition);

            //trace('${transition.targetID} ' + trans.length);
        }
    }

    public function getTransition(actorId:Int, targetId:Int):TransitionData{

        var transitionsByTargetId = transitionsByActorIdTargetId[actorId];
        if(transitionsByTargetId != null) {
            trace("tt " + transitionsByTargetId[targetId]);
            return transitionsByTargetId[targetId];
        }
        return null;
    }
}