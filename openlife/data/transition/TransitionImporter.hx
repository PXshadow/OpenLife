package openlife.data.transition;

import openlife.settings.ServerSettings;
import openlife.data.object.ObjectHelper;
import openlife.data.object.ObjectData;
import haxe.io.Path;
import sys.io.File;
import openlife.engine.Engine;
@:expose
class TransitionImporter
{
    public static var transitionMap:Map<Int, ObjectData> = [];
    public static var transitionImporter:TransitionImporter = new TransitionImporter();

    public var transitions:Array<TransitionData> = [];
    public var categories:Array<Category> = [];
    private var categoriesById:Map<Int, Category> = [];

    // for reverse lookup of transitions
    private var transitionsByNewTargetMap:Map<Int, Array<TransitionData>>;
    private var transitionsByNewActorMap:Map<Int, Array<TransitionData>>;

    // transitions without any last use transitions
    private var transitionsByActorIdTargetId:Map<Int, Map<Int, TransitionData>>;
    // transitions where both actor and target is last use
    private var lastUseBothTransitionsByActorIdTargetId:Map<Int, Map<Int, TransitionData>>; // maybe no need
    // transitions where only actor is last use
    private var lastUseActorTransitionsByActorIdTargetId:Map<Int, Map<Int, TransitionData>>; // maybe no need
    // transitions where only target is last use
    private var lastUseTargetTransitionsByActorIdTargetId:Map<Int, Map<Int, TransitionData>>;

    // maxUseTransitions
    private var maxUseTransitionsByActorIdTargetId:Map<Int, Map<Int, TransitionData>>;



    public function new()
    {

    }

    public static function DoAllInititalisationStuff()
    {
        trace("Import transitions...");
        TransitionImporter.transitionImporter.importCategories();
        TransitionImporter.transitionImporter.importTransitions();
        TransitionImporter.transitionImporter.setParentFoods();

        ServerSettings.PatchTransitions(TransitionImporter.transitionImporter);
    }

    public function setParentFoods()
    {
        for(food in ObjectData.foodObjects)
        {
            var transData = TransitionImporter.GetTransitionByNewActor(food.id);

            for(trans in transData)
            {
                // food target with empty hand like wild onion or berrybush
                if(trans.targetID > 0 && trans.actorID == 0)
                {
                    var obj = ObjectData.getObjectData(trans.targetID);

                    obj.foodFromTarget = food;    

                    //trace('Food Target: ${food.description} actor: ${trans.actorID} <-- ${obj.description} ${obj.id}');
                }
                // TODO food target with tool like sharpstone
                // For cooking like transitions: Cooked Garlic Shrimp actor: 4324 <-- Hot Coals# +tool 85
                if(trans.targetID > 0 && trans.actorID != 0)
                {
                    var targetObj = ObjectData.getObjectData(trans.targetID);
                    var actorObj = ObjectData.getObjectData(trans.actorID);

                    targetObj.foodFromTargetWithTool = food;    
                    actorObj.foodFromActor = food;   

                    trace('Food Target With Tool: ${food.description} <-- ${actorObj.id} ${actorObj.description} + ${targetObj.id} ${targetObj.description} ');
                }
            }           
        }
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

        transitionsByNewTargetMap = [];
        transitionsByNewActorMap = []; 

        transitions = [];
        transitionsByActorIdTargetId = [];
        lastUseBothTransitionsByActorIdTargetId = [];
        lastUseActorTransitionsByActorIdTargetId = [];
        lastUseTargetTransitionsByActorIdTargetId = [];

        maxUseTransitionsByActorIdTargetId = [];

        for (name in sys.FileSystem.readDirectory(Engine.dir + "transitions"))
        {
            
            var transition = TransitionData.createNewFromFile(Path.withoutExtension(name),File.getContent(Engine.dir + 'transitions/$name'));
            
            addTransition("importTransitions: ", transition);

            createAndaddCategoryTransitions(transition); 

        }

        trace('Transitions loaded: ${transitions.length}');
    }

    private function getTransitionMap(lastUseActor:Bool, lastUseTarget:Bool, maxUseTarget:Bool=false):Map<Int, Map<Int, TransitionData>>
    {
        if(maxUseTarget) return maxUseTransitionsByActorIdTargetId;

        if(lastUseActor && lastUseTarget)
        {
            return lastUseBothTransitionsByActorIdTargetId;
        }
        else if(lastUseActor && lastUseTarget == false)
        {
            return lastUseActorTransitionsByActorIdTargetId;
        }
        else if(lastUseActor == false && lastUseTarget)
        {
            return lastUseTargetTransitionsByActorIdTargetId;
        } 
        else
        {
            return transitionsByActorIdTargetId;
        }
    }

    private function getTransitionMapByTargetId(id:Int, lastUseActor:Bool, lastUseTarget:Bool, maxUseTarget:Bool=false):Map<Int, TransitionData>
    {
        var transitionMap = getTransitionMap(lastUseActor, lastUseTarget, maxUseTarget);

        var transitionsByTargetId = transitionMap[id];

        if(transitionsByTargetId == null){
            transitionsByTargetId = [];
            transitionMap[id] = transitionsByTargetId;
        }

        return transitionsByTargetId;
    }

    public static function GetTrans(actor:ObjectHelper, target:ObjectHelper):TransitionData
    {
        return transitionImporter.getTrans(actor, target);
    }

    public function getTrans(actor:ObjectHelper, target:ObjectHelper):TransitionData
    {
        // actor last use is handled through actor + -1 = newActor + 0 transitions
        var transition = getTransition(actor.parentId, target.parentId, false, target.isLastUse());

        // 58 + 139 // thread + skwer --> skewer does not seem to have a last use transtion, so if none found, 
        if(transition == null) transition = getTransition(actor.parentId, target.parentId, false, false); // this might make errors... 

        return transition;
    }

    public static function GetTransition(actorId:Int, targetId:Int, lastUseActor:Bool = false, lastUseTarget:Bool = false, maxUseTarget:Bool=false):TransitionData
    {
        return transitionImporter.getTransition(actorId, targetId, lastUseActor, lastUseTarget, maxUseTarget);
    }

    public function getTransition(actorId:Int, targetId:Int, lastUseActor:Bool = false, lastUseTarget:Bool = false, maxUseTarget:Bool=false):TransitionData
    {
        var objDataActor = ObjectData.getObjectData(actorId);
        var objDataTarget = ObjectData.getObjectData(targetId);

        if(objDataActor.dummyParent != null) objDataActor = objDataActor.dummyParent;
        if(objDataTarget.dummyParent != null) objDataTarget = objDataTarget.dummyParent;

        //if(objDataActor.id != -1 && objDataTarget.id != -1) trace('getTransition: ${objDataActor.id} + ${objDataTarget.id} lastUseTarget: $lastUseTarget maxUseTarget: $maxUseTarget');

        var transitionMap = getTransitionMap(lastUseActor, lastUseTarget, maxUseTarget);

        var transitionsByTargetId = transitionMap[objDataActor.id];

        if(transitionsByTargetId == null) return null;

        return transitionsByTargetId[objDataTarget.id];
    }

    public static function GetTransitionByNewTarget(newTargetId:Int) : Array<TransitionData>
    {
        return transitionImporter.getTransitionByNewTarget(newTargetId);
    }

    public function getTransitionByNewTarget(newTargetId:Int) : Array<TransitionData>
    {
        var transitions = transitionsByNewTargetMap[newTargetId];

        return transitions != null ? transitions : new Array<TransitionData>(); 
    }

    public static function GetTransitionByNewActor(newActorId:Int) : Array<TransitionData>
    {
        return transitionImporter.getTransitionByNewActor(newActorId);
    }

    public function getTransitionByNewActor(newActorId:Int) : Array<TransitionData>
    {
        var transitions = transitionsByNewActorMap[newActorId];

        return transitions != null ? transitions : new Array<TransitionData>(); 
    }

    public function addTransition(addedBy:String, transition:TransitionData,  lastUseActor:Bool = false, lastUseTarget:Bool = false)
    {
        transition.addedBy = addedBy;

        if(lastUseActor == false && lastUseTarget == false){
            // if transition is a reverse transition, it can be done also on lastUse Items so add Transaction for that
            if(transition.lastUseActor == false && transition.reverseUseActor && transition.lastUseTarget == false && transition.reverseUseTarget) addTransition("reverseUseActor & reverseUseTarget", transition, true, true);
            else if(transition.lastUseActor == false && transition.reverseUseActor) addTransition('$addedBy-reverseUseActor', transition, true, transition.lastUseTarget);
            else if(transition.lastUseTarget == false && transition.reverseUseTarget) addTransition('$addedBy-reverseUseTarget', transition, transition.lastUseActor, true);
        }
        else{
            transition = transition.clone();
            if(lastUseActor) transition.lastUseActor = true;
            if(lastUseTarget) transition.lastUseTarget = true;
        }
       
        var transitionsByTargetId = getTransitionMapByTargetId(transition.actorID, transition.lastUseActor, transition.lastUseTarget);
        var trans = transitionsByTargetId[transition.targetID];
        
        if(trans == null)
        {
            this.transitions.push(transition);
            transitionsByTargetId[transition.targetID] = transition;
            addTransitionByNewTarget(transition); 
            addTransitionByNewActor(transition);           

            transition.traceTransition(addedBy);

            //if(transition.reverseUseTarget) traceTransition(transition, "", "");
            return;
        }

        // handle double transitions

        // add max use transitions that change target like in case of a well site with max stones or Canada Goose Pond with max
        // 33 + 1096 = 0 + 1096 targetRemains: true
        // 33 + 1096 = 0 + 3963 targetRemains: false
        if(trans.targetRemains && transition.targetRemains == false)
        {
            trans.traceTransition('$addedBy 1maxUseTransition targetRemains true: ');
            transition.traceTransition('$addedBy 1maxUseTransition targetRemains: false: ');

            var maxUseTransitionsByTargetId = getTransitionMapByTargetId(transition.actorID, false, false, true);
            maxUseTransitionsByTargetId[transition.targetID] = transition;

            this.transitions.push(transition);
            addTransitionByNewTarget(transition); 
            addTransitionByNewActor(transition);

            return;
        }

        if(trans.targetRemains == false && transition.targetRemains)
        {
            transition.traceTransition( '$addedBy 2maxUseTransition targetRemains: true');
            trans.traceTransition('$addedBy 2maxUseTransition targetRemains: false:');

            var maxUseTransitionsByTargetId = getTransitionMapByTargetId(transition.actorID, false, false, true);

            this.transitions.push(transition);
            addTransitionByNewTarget(transition); 
            addTransitionByNewActor(transition);

            transitionsByTargetId[transition.targetID] = transition;
            maxUseTransitionsByTargetId[trans.targetID] = trans;

            return;
        }

        // TODO there are a lot of double transactions, like Oil Movement, Horse Stuff, Fence / Wall Alignment, Rose Seed
        trans.traceTransition('$addedBy WARNING DOUBLE 1!!');
        transition.traceTransition('$addedBy WARNING DOUBLE 2!!');
    }

    private function addTransitionByNewTarget(transition:TransitionData)
    {
        var transitionsByNewTarget = transitionsByNewTargetMap[transition.newTargetID];

        if(transitionsByNewTarget == null)
        {
            transitionsByNewTargetMap[transition.newTargetID] = new Array<TransitionData>();
            transitionsByNewTarget = transitionsByNewTargetMap[transition.newTargetID];
        }

        transitionsByNewTarget.push(transition);
    }

    private function addTransitionByNewActor(transition:TransitionData)
    {
        var transitions = transitionsByNewActorMap[transition.newActorID];

        if(transitions == null)
        {
            transitionsByNewActorMap[transition.newActorID] = new Array<TransitionData>();
            transitions = transitionsByNewActorMap[transition.newActorID];
        }

        transitions.push(transition);
    }

    

    // seems like obid can be at the same time a category and an object / Cabbage Seed + Bowl of Cabbage Seeds / 1206 + 1312
    // so better look also for @ in the description
    public function getCategory(id:Int) : Category
    {
        //var objectData = ObjectData.getObjectData(id);

        //if(objectData == null) return null;

        // TODO there was a reason for checking for @, but 1206 (Cabbage Seed) is an object and an category so wont work for this
        //if(objectData.description.indexOf("@") == -1) return null;

        return categoriesById[id];
    }

    public function createAndaddCategoryTransitions(transition:TransitionData){

        var actorCategory = getCategory(transition.actorID);
        var targetCategory = getCategory(transition.targetID);
        var newActorCategory = getCategory(transition.newActorID);
        var newTargetCategory = getCategory(transition.newTargetID);

        var bothActionAndTargetIsCategory = (actorCategory != null) && (targetCategory != null);

        if(bothActionAndTargetIsCategory)
        {
            // TODO many strange transitions to look at...
            //traceTransition(transition, 'bothActionAndTargetIsCategory: ');
            //return;
        }

        if((actorCategory != null && actorCategory.pattern) || (targetCategory != null && targetCategory.pattern)) 
        {
            //trace('Pattern: ${actorCategory.parentID}');
            /*if(actorCategory != null)
            {
                if(actorCategory.parentID == 1206) trace('Pattern: actorCategory.ids.length: ${actorCategory.ids.length}');
                if(actorCategory.parentID == 1206 && targetCategory != null) trace('Pattern: targetCategory.ids.length: ${targetCategory.ids.length}');
                if(actorCategory.parentID == 1206 && newActorCategory != null) trace('Pattern: newActorCategory.ids.length: ${newActorCategory.ids.length}');
                if(actorCategory.parentID == 1206 && newTargetCategory != null) trace('Pattern: newTargetCategory.ids.length: ${newTargetCategory.ids.length}');
            }*/

            var description = "Pattern: ";
            if(actorCategory != null)
            {
                var objData = ObjectData.getObjectData(actorCategory.parentID);
                description += objData == null ? '${actorCategory.parentID} ' : '${actorCategory.parentID} ${objData.description} ';
            }

            if(targetCategory != null)
            {
                var objData = ObjectData.getObjectData(targetCategory.parentID);
                description += objData == null ? '${targetCategory.parentID} ' : '${targetCategory.parentID} ${objData.description} ';
            }

            var length = actorCategory != null ? actorCategory.ids.length : targetCategory.ids.length;

            for(i in 0...length)
            { 
                var newTransition = transition.clone();
                if(actorCategory != null) newTransition.actorID = actorCategory.ids[i];
                if(targetCategory != null) newTransition.targetID = targetCategory.ids[i];
                if(newActorCategory != null) newTransition.newActorID = newActorCategory.ids[i];
                if(newTargetCategory != null) newTransition.newTargetID = newTargetCategory.ids[i];

                //newTransition.traceTransition('$description/', true);
                addTransition('$description/', newTransition);
            }

            return;
        }

        /*     
        // add some pile target transitions
        // 1601 + 1600 = 0 + 1600
        // 1601 + 1601
        // 0 + 1600 = 1601 + 1601 (last)
        // 0 + 1600 = 1601 + 1600 
        var isPileTransition = targetCategory != null && (targetCategory.parentID == 1600 || targetCategory.parentID == 1601);
        isPileTransition = isPileTransition || (actorCategory != null && (actorCategory.parentID == 1600 || actorCategory.parentID == 1601));

        if(isPileTransition) 
        {
            //trace(transition);

            var pileCategory = getCategory(1600);
            var pileItemsCategory = getCategory(1601);

            for(i in 0...pileCategory.ids.length)
            {
                var newTransition = transition.clone();

                if(newTransition.actorID == 1601) newTransition.actorID = pileItemsCategory.ids[i];
                if(newTransition.targetID == 1600) newTransition.targetID = pileCategory.ids[i];
                else if(newTransition.targetID == 1601) newTransition.targetID = pileItemsCategory.ids[i];
                
                if(newTransition.newActorID == 1601) newTransition.newActorID = pileItemsCategory.ids[i];
                if(newTransition.newTargetID == 1600) newTransition.newTargetID = pileCategory.ids[i];
                else if(newTransition.newTargetID == 1601) newTransition.newTargetID = pileItemsCategory.ids[i];

                addTransition("Pile: ", newTransition);
            }
            
            return;
        }
        */

        var category = actorCategory;
        

        if(category != null)
        {
            var objData = ObjectData.getObjectData(category.parentID);
            var categoryDesc = objData == null ? '${category.parentID}' : '${category.parentID} ${objData.description}';

            for(id in category.ids)
            {                 
                if(targetCategory == null)
                {
                    var newTransition = transition.clone();
                    newTransition.actorID = id;
                    if(newTransition.newActorID == category.parentID) newTransition.newActorID = id;

                    addTransition('Actor Category: ${categoryDesc} Trans:', newTransition);
                } 
                // TODO both category may not be needed
                else{
                    for(targetId in targetCategory.ids){

                        var newTransition = transition.clone();
                        newTransition.actorID = id;
                        newTransition.targetID = targetId;

                        if(newTransition.newActorID == category.parentID) newTransition.newActorID = id;
                        if(newTransition.newTargetID == targetCategory.parentID) newTransition.newTargetID = targetId;

                        addTransition("Both Category: ", newTransition);
                        //traceTransition(newTransition, 'bothActionAndTargetIsCategory: ');

                    }
                }
            }
        }

        if(bothActionAndTargetIsCategory) return;

        // for transitions where actor is no category but target is a category
        category = targetCategory;

        if(category != null)
        {
            var objData = ObjectData.getObjectData(category.parentID);
            var categoryDesc = objData == null ? '${category.parentID}' : '${category.parentID} ${objData.description}';

            for(id in category.ids)
            {
                var newTransition = transition.clone();
                newTransition.targetID = id;
                if(newTransition.newTargetID == category.parentID) newTransition.newTargetID = id;

                addTransition('Target Category ${categoryDesc}: ', newTransition); 
            }
        }
    }
}