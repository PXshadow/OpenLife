package openlife.data.transition;

import openlife.server.Server;
import openlife.data.object.ObjectData;
import haxe.io.Path;
import sys.io.File;
import openlife.engine.Engine;
@:expose
class TransitionImporter
{
    public var transitions:Array<TransitionData> = [];
    public var categories:Array<Category> = [];

    private var transitionsByActorIdTargetId:Map<Int, Map<Int, TransitionData>>;
    private var lastTransitionsByActorIdTargetId:Map<Int, Map<Int, TransitionData>>;

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
        lastTransitionsByActorIdTargetId = [];

        for (name in sys.FileSystem.readDirectory(Engine.dir + "transitions"))
        {
            
            var transition = TransitionData.createNewFromFile(Path.withoutExtension(name),File.getContent(Engine.dir + 'transitions/$name'));
            
            addTransition(transition);

        }

        trace('Transitions loaded: ${transitions.length}');
    }

    public function traceTransition(transition:TransitionData){

        var objectDataActor = Server.objectDataMap[transition.actorID];
        var objectDataTarget = Server.objectDataMap[transition.targetID];

        var actorDescription = "";
        var targetDescription = "";

        if(objectDataActor != null) actorDescription = objectDataActor.description;
        if(objectDataTarget != null) targetDescription = objectDataTarget.description;
        
        trace('New transition: a: ${transition.actorID} t: ${transition.targetID} - $actorDescription - $targetDescription');
    }

    public function generateAndAddCategoryTransitions(){

        if(transitions.length == 0) importTransitions();
        if(categories.length == 0) importCategories();

        var count = 0;

        // TODO currently it adds only transitions if category is actor

        for(category in categories){

            // TODO do also for last transitions

            var transitionsByTargetId = transitionsByActorIdTargetId[category.parentID];

            if(transitionsByTargetId == null){
                trace('no action found for category: ${category}');
                continue;
            }

            for(transition in transitionsByTargetId){

                for(id in category.ids){
                    
                    var newTransition = transition.clone();
                    newTransition.actorID = id;
                    if(newTransition.newActorID == category.parentID) newTransition.newActorID = id;
                    if(newTransition.newTargetID == category.parentID) newTransition.newTargetID = id;

                    addTransition(newTransition);                   

                    count++;
                }
            }
        }

        trace('added category transitions: $count');
    }

    public function addTransition(transition:TransitionData){

        var transitionsByTargetId;

        // there is one map for last transitions
        if(transition.lastUseActor){
            transitionsByTargetId = lastTransitionsByActorIdTargetId[transition.actorID];
        }
        else{
            transitionsByTargetId = transitionsByActorIdTargetId[transition.actorID];
        }
        
        if(transitionsByTargetId == null){
            transitionsByTargetId = [];
            transitionsByActorIdTargetId[transition.actorID] = transitionsByTargetId;
        }

        var trans = transitionsByTargetId[transition.targetID];
        
        // if there is a transition allready, then there is an additional "last" transition
        if(trans != null){
            // TODO make map for last transitions
            trace('Double transition: actor: ${trans.actorID} target: ${trans.targetID}');
        }

        this.transitions.push(transition);
        transitionsByTargetId[transition.targetID] = transition;

        traceTransition(transition);
    }

    

    public function getTransition(actorId:Int, targetId:Int):TransitionData{

        var transitionsByTargetId = transitionsByActorIdTargetId[actorId];

        if(transitionsByTargetId == null) return null;

        return transitionsByTargetId[targetId];
    }
}