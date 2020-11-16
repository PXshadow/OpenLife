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
    private var categoriesById:Map<Int, Category> = [];

    private var transitionsByActorIdTargetId:Map<Int, Map<Int, TransitionData>>;
    private var lastTransitionsByActorIdTargetId:Map<Int, Map<Int, TransitionData>>;

    public function new()
    {

    }

    public function importCategories()
    {
        categories = [];
        categoriesById = [];

        for (name in sys.FileSystem.readDirectory(Engine.dir + "categories"))
        {
            var category = new Category(File.getContent(Engine.dir + 'categories/$name'));
            
            categories.push(category);
            categoriesById[category.parentID] = category;

        }
    }

    public function importTransitions()
    {        
        if(categories.length == 0) importCategories();

        transitions = [];
        transitionsByActorIdTargetId = [];
        lastTransitionsByActorIdTargetId = [];

        for (name in sys.FileSystem.readDirectory(Engine.dir + "transitions"))
        {
            
            var transition = TransitionData.createNewFromFile(Path.withoutExtension(name),File.getContent(Engine.dir + 'transitions/$name'));
            
            addTransition(transition);

            createAndaddCategoryTransitions(transition); 

        }

        trace('Transitions loaded: ${transitions.length}');
    }

    public function traceTransition(transition:TransitionData, s:String = ""){

        var objectDataActor = Server.objectDataMap[transition.actorID];
        var objectDataTarget = Server.objectDataMap[transition.targetID];

        var actorDescription = "";
        var targetDescription = "";

        if(objectDataActor != null) actorDescription = objectDataActor.description;
        if(objectDataTarget != null) targetDescription = objectDataTarget.description;
        
        trace('$s transition: a: ${transition.actorID} t: ${transition.targetID} - $actorDescription - $targetDescription');
    }

    public function createAndaddCategoryTransitions(transition:TransitionData){

        var actorCategory = this.categoriesById[transition.actorID];
        var targetCategory = this.categoriesById[transition.targetID];
        var bothActionAndTargetIsCategory = (actorCategory != null) && (targetCategory != null);

        if(bothActionAndTargetIsCategory){
            traceTransition(transition, 'bothActionAndTargetIsCategory: ');
        }

        var category = actorCategory;

        if(category != null){
            for(id in category.ids){

                if(targetCategory == null){
                    var newTransition = transition.clone();
                    newTransition.actorID = id;
                    if(newTransition.newActorID == category.parentID) newTransition.newActorID = id;

                    addTransition(newTransition);
                } 
                else{
                    for(targetId in targetCategory.ids){

                        var newTransition = transition.clone();
                        newTransition.actorID = id;
                        newTransition.targetID = targetId;

                        if(newTransition.newActorID == category.parentID) newTransition.newActorID = id;
                        if(newTransition.newTargetID == targetCategory.parentID) newTransition.newTargetID = targetId;

                        addTransition(newTransition);
                        //traceTransition(newTransition, 'bothActionAndTargetIsCategory: ');

                    }
                }
            }
        }

        if(bothActionAndTargetIsCategory) return;

        // for transitions where actor is no category but target is a category
        category = targetCategory;

        if(category != null){
            for(id in category.ids){

                var newTransition = transition.clone();
                newTransition.targetID = id;
                if(newTransition.newTargetID == category.parentID) newTransition.newTargetID = id;

                addTransition(newTransition); 
            }
        }
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

        //traceTransition(transition);
    }

    

    public function getTransition(actorId:Int, targetId:Int):TransitionData{

        var transitionsByTargetId = transitionsByActorIdTargetId[actorId];

        if(transitionsByTargetId == null) return null;

        return transitionsByTargetId[targetId];
    }
}