package openlife.data.transition;

import haxe.Exception;
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

    private var allTransitionsByTargetMap:Map<Int, Array<TransitionData>>;
    private var allTransitionsByActorMap:Map<Int, Array<TransitionData>>;

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

    public function new(){}

    public static function DoAllInititalisationStuff()
    {
        trace("Import transitions...");
        TransitionImporter.transitionImporter.importCategories();
        TransitionImporter.transitionImporter.importTransitions();
        TransitionImporter.transitionImporter.setParentFoods();

        ServerSettings.PatchTransitions(TransitionImporter.transitionImporter);

        TransitionImporter.transitionImporter.calculateCraftingSteps();
        TransitionImporter.SortFood();        
    }

    public static function SortFood()
    {
        //var players = [for(p in AllPlayers) p];

        //if(players.length < 2) return PrestigeClass.Commoner;

        var foods = ObjectData.foodObjects;

        foods.sort(function(a, b) {
            if(b.carftingSteps < 0) return -1; 
            if(a.carftingSteps < 0) return 1; 
            if(a.carftingSteps < b.carftingSteps) return -1;
            else if(a.carftingSteps > b.carftingSteps) return 1;
            else return 0;
        });

        var done = 0;
        var notDone = 0;

        for(food in ObjectData.foodObjects)
        {
            trace('Food: steps: ${food.carftingSteps} id: ${food.id} ${food.description}');

            if(food.carftingSteps < 0) notDone++;
            else done++;
        }

        trace('Steps: ALL Food: done: $done notDone: $notDone');
    }

    public function calculateCraftingSteps()
    {
        // TODO 1869 Wet Mango Sapling Cutting
        // TODO 3375 Feast Plate
        var todo = new Array<ObjectData>();
        
        var steps = 0;

        var obj = ObjectData.getObjectData(0);
        obj.carftingSteps = 0;
        var obj = ObjectData.getObjectData(-1);
        obj.carftingSteps = 0;
        var obj = ObjectData.getObjectData(3962); // Loose Muddy Iron Vein
        obj.carftingSteps = 0;
        todo.push(obj);

        for (obj in ObjectData.importedObjectData)
        {
            if(obj.isNatural() == false) continue;
            
            //trace('Natural: id: ${obj.id} ${obj.description}');

            obj.carftingSteps = 0;

            todo.push(obj);
        }

        var todo2 = todo;
        var done = 0;

        while(todo2.length > 0)
        {
            done += todo2.length;
            //trace('Steps: $steps done: $done d: ${todo2.length}');

            todo = todo2;
            todo2 = new Array<ObjectData>();
            steps += 1;

            for (obj in todo)
            {
                var transitionsByActor = allTransitionsByActorMap[obj.id];
                if(transitionsByActor == null) continue;

                for(trans in transitionsByActor)
                {
                    var target = ObjectData.getObjectData(trans.targetID);
                    if(target.carftingSteps < 0) continue;
                    calculateCraftingStepsHelper(todo2, obj, trans, steps);
                }
            }
            
            for (obj in todo)
            {
                var transitionsByTarget = allTransitionsByTargetMap[obj.id];
                if(transitionsByTarget == null) continue;

                for(trans in transitionsByTarget)
                {
                    var actor = ObjectData.getObjectData(trans.actorID);
                    if(actor.carftingSteps < 0) continue;
                    calculateCraftingStepsHelper(todo2, obj, trans, steps);
                }                
            }
        }
    
        trace('Steps: ALL Steps: $steps done: $done ');

        var done = 0;
        var notDone = 0;

        for(obj in ObjectData.importedObjectData)
        {
            trace('Obj: steps: ${obj.carftingSteps} id: ${obj.id} ${obj.description}');

            if(obj.carftingSteps < 0) notDone++;
            else done++;
        }

        trace('Steps: ALL Obj: done: $done notDone: $notDone');
    }

    private function calculateCraftingStepsHelper(newTodo:Array<ObjectData>, obj:ObjectData, trans:TransitionData, steps:Int)
    {
        AddObject(newTodo, obj, trans.newActorID, steps);
        AddObject(newTodo, obj, trans.newTargetID, steps);

        var newTargetCategory = getCategory(trans.newTargetID);
        if(newTargetCategory == null || newTargetCategory.probSet == false) return;

        // time transitions with other endings like Blooming Squash Plant // Ripe Pumpkin Plant
        // like 3221 Perhaps a Pumpkin 
        for(i in 0...newTargetCategory.ids.length)
        {
            var id = newTargetCategory.ids[i];
            AddObject(newTodo, obj, id, steps);
        }
    }

    private static function AddObject(objects:Array<ObjectData>, from:ObjectData, objId:Int, steps:Int)
    {
        var newObj = ObjectData.getObjectData(objId);
        if(newObj.carftingSteps >= 0) return;

        newObj.carftingSteps = steps;
        objects.push(newObj);

        //trace('Steps: $steps id: ${from.id} ${from.description} -->  ${newObj.id} ${newObj.description}');
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

                    //trace('Food Target With Tool: ${food.description} <-- ${actorObj.id} ${actorObj.description} + ${targetObj.id} ${targetObj.description} ');
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
            //if(category.probSet) trace('Category: ${category.parentID}');
        }
    }

    public function importTransitions()
    {        
        if(categories.length == 0) importCategories();

        allTransitionsByActorMap = [];
        allTransitionsByTargetMap = []; 

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
        // skip if transition does nothing like <560> + <0> = <560> + <0> / Knife#
        if(transition.actorID == transition.newActorID && transition.targetID == transition.newTargetID) return;

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

            addTransitionToMap(allTransitionsByActorMap, transition, transition.actorID);
            addTransitionToMap(allTransitionsByTargetMap, transition, transition.targetID);
            addTransitionToMap(transitionsByNewActorMap, transition, transition.newActorID);
            addTransitionToMap(transitionsByNewTargetMap, transition, transition.newTargetID);
         
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
            addTransitionToMap(allTransitionsByActorMap, transition, transition.actorID);
            addTransitionToMap(allTransitionsByTargetMap, transition, transition.targetID);
            addTransitionToMap(transitionsByNewActorMap, transition, transition.newActorID);
            addTransitionToMap(transitionsByNewTargetMap, transition, transition.newTargetID);

            return;
        }

        if(trans.targetRemains == false && transition.targetRemains)
        {
            transition.traceTransition( '$addedBy 2maxUseTransition targetRemains: true');
            trans.traceTransition('$addedBy 2maxUseTransition targetRemains: false:');

            var maxUseTransitionsByTargetId = getTransitionMapByTargetId(transition.actorID, false, false, true);

            this.transitions.push(transition);
            addTransitionToMap(allTransitionsByActorMap, transition, transition.actorID);
            addTransitionToMap(allTransitionsByTargetMap, transition, transition.targetID);
            addTransitionToMap(transitionsByNewActorMap, transition, transition.newActorID);
            addTransitionToMap(transitionsByNewTargetMap, transition, transition.newTargetID);

            transitionsByTargetId[transition.targetID] = transition;
            maxUseTransitionsByTargetId[trans.targetID] = trans;

            return;
        }

        // TODO there are a lot of double transactions, like Oil Movement, Horse Stuff, Fence / Wall Alignment, Rose Seed
        trans.traceTransition('$addedBy WARNING DOUBLE 1!!');
        transition.traceTransition('$addedBy WARNING DOUBLE 2!!');
    }

    private function addTransitionToMap(map:Map<Int, Array<TransitionData>>, transition:TransitionData, key:Int)
    {
        var array = map[key];

        if(array == null)
        {
            map[key] = new Array<TransitionData>();
            array = map[key];
        }

        //if(transition.newActorID == 34) trace('add Trans: ' + transition.getDesciption(true));

        array.push(transition);
    }


    

    // seems like obid can be at the same time a category and an object / Cabbage Seed + Bowl of Cabbage Seeds / 1206 + 1312
    // so better look also for @ in the description
    public function getCategory(id:Int) : Category
    {
        //var objectData = ObjectData.getObjectData(id);

        //if(objectData == null) return null;

        // TODO there was a reason for checking for @, but 1206 (Cabbage Seed) is an object and an category so wont work for this
        //if(objectData.description.indexOf("@") == -1) return null;

        // For example 2982 is Shaky property Fence  and a category so wont work for this if testing not for @. like knife + Shaky property Fenc

        return categoriesById[id];
    }

    public function createAndaddCategoryTransitions(transition:TransitionData){

        var actorCategory = getCategory(transition.actorID);
        var targetCategory = getCategory(transition.targetID);
        var newActorCategory = getCategory(transition.newActorID);
        var newTargetCategory = getCategory(transition.newTargetID);
        var bothActorAndTargetIsCategory = (actorCategory != null) && (targetCategory != null);

        // Category tansitions with different outcome
        // <-1> + <1195> = <0> + <3221> / TIME + Blooming Squash Plant -->  EMPTY + Perhaps a Pumpkin
        if(newTargetCategory != null && newTargetCategory.probSet)
        {
            var objCategory = ObjectData.getObjectData(newTargetCategory.parentID);
            //trace('PROB Category: ' + transition.getDesciption());
            addTransition('Prob/${objCategory.name}/', transition);

            for(i in 0...newTargetCategory.ids.length)
            {
                var id = newTargetCategory.ids[i];
                var weight = newTargetCategory.weights[i];
                var newTransition = transition.clone();
                
                trace('PROB Category: $id weight: ${weight} ' + transition.getDesciption());
                
                newTransition.newTargetID = newTargetCategory.parentID;
                //addTransition('Prob/${objCategory.name}/', newTransition);
                addTransitionToMap(transitionsByNewActorMap, newTransition, newTransition.newActorID);
                addTransitionToMap(transitionsByNewTargetMap, newTransition, newTransition.newTargetID);
            }
            
            return;
        }

        if(bothActorAndTargetIsCategory)
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
                description += objData == null ? 'A: ${actorCategory.parentID} ' : 'A: ${actorCategory.parentID} ${objData.description} ';
            }

            if(targetCategory != null)
            {
                var objData = ObjectData.getObjectData(targetCategory.parentID);
                description += objData == null ? 'T: ${targetCategory.parentID} ' : 'T: ${targetCategory.parentID} ${objData.description} ';
            }

            var length = actorCategory != null ? actorCategory.ids.length : targetCategory.ids.length;

            //if(actorCategory != null && actorCategory.parentID == 2995 ) trace('TTT11: ${description}' + transition.getDesciption());

            // FIX: This is not a pattern: A: 2995 @ Shaky Fence Buster T: 2982 Shaky Property Fence# +horizontalC <2995> + <2982> 
            if(actorCategory != null && actorCategory.pattern == false) targetCategory = null; // TODO loop over other category? But when, since Shaky Property Fence is an object and a total different category
            if(targetCategory != null && targetCategory.pattern == false) actorCategory = null; // TODO loop over other category? 

            if(actorCategory != null && targetCategory != null && actorCategory.ids.length != targetCategory.ids.length)
            {
                trace('WARNING: ${description}' + transition.getDesciption());
                throw new Exception('actorCategory.ids.length != targetCategory.ids.length');
            }

            for(i in 0...length)
            { 
                var newTransition = transition.clone();
                if(actorCategory != null) newTransition.actorID = actorCategory.ids[i];
                if(targetCategory != null) newTransition.targetID = targetCategory.ids[i];
                if(newActorCategory != null) newTransition.newActorID = newActorCategory.ids[i];
                if(newTargetCategory != null) newTransition.newTargetID = newTargetCategory.ids[i];

                //if(actorCategory != null && actorCategory.parentID == 2995 ) trace('TTT12: ' + newTransition.getDesciption());
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
                    //if(category.parentID == 1127) trace ('CATEGORY TRANS: ' + transition.getDesciption()); 

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

        if(bothActorAndTargetIsCategory) return;

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