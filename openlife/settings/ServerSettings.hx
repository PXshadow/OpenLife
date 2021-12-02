package openlife.settings;
import openlife.server.TransitionHelper;
import openlife.server.Server;
import sys.FileSystem;
import haxe.macro.Expr.Catch;
import sys.io.File;
import openlife.data.transition.TransitionData;
import openlife.data.transition.TransitionImporter;
import openlife.server.Biome.BiomeTag;
import openlife.data.object.ObjectData;

@:rtti
class ServerSettings
{
    // DEBUG: switch on / off
    public static var dumpOutput = false;
    public static var debug = false; // activates or deactivates try catch blocks and initial debug objects generation // deactivates saving
    public static var saveToDisk = true; 
    public static var AllowDebugCommmands = true; // can create objects with saying "create ID" / "create object" "create object!" with ! indicating that object ends with "object" or test wounds with using "hit" or "heal"
    public static var DebugWrite = false; // WordMap writeToDisk
    public static var TraceCountObjects = false; // WorldMap
    public static var DebugSpeed = false; // MovementHelper
    
    // DEBUG: used to trace connection.send commands 
    public static var TraceSendPlayerActions = false;  //  only trace player actions // ignores MX from animal, FX and PU from food / age
    public static var TraceSendNonPlayerActions = false;  //  only trace non player actions // traces only MX from animal, FX and PU from food / age

    // DEBUG: TransitionImporter // for debugging transitions
    public static var traceTransitionByActorId = 9992710; // set to object id which you want to debug
    public static var traceTransitionByActorDescription = "!!!Basket of Soil"; // set to object description which you want to debug
    public static var traceTransitionByTargetId = 9992710; // set to object id which you want to debug
    public static var traceTransitionByTargetDescription = "!!!Basket of Soil"; // set to object description which you want to debug

    // PlayerInstance
    public static var ChanceForFemaleChild = 0.6;
    public static var MaxPlayersBeforeStartingAsChild = -1; // -1
    public static var StartingEveAge = 14;  // 14
    public static var StartingFamilyName = "Snow";
    public static var StartingName = "Spoonwood";
    public static var AgingSecondsPerYear = 60; // 60
    public static var ReduceAgeNeededToPickupObjects = 3; // reduces the needed age that an item can be picked up. But still it cant be used if age is too low
    public static var MaxAgeForAllowingClothAndPrickupFromOthers = 10;
    public static var MaxChildAgeForBreastFeeding = 6;
    public static var MinAgeFertile = 12;
    public static var MaxAgeFertile = 42;
    
    // save to disk
    public static var TicksBetweenSaving = 200;
    public static var TicksBetweenBackups = 20 * 60 * 60 * 8; // 20 * 60 * 60 * 8 = every 8 hours
    public static var MaxNumberOfBackups = 90;

    public static var MapFileName = "mysteraV1Test.png";  
    public static var SaveDirectory = "SaveFiles";
    public static var OriginalBiomesFileName    =   "OriginalBiomes";   // .bin is added
    public static var CurrentBiomesFileName     =   "CurrentBiomes";    // .bin is added
    public static var CurrentFloorsFileName     =   "CurrentFloors";    // .bin is added
    public static var OriginalObjectsFileName   =   "OriginalObjects";  // .bin is added
    public static var CurrentObjectsFileName    =   "CurrentObjects";   // .bin is added
    public static var CurrentObjHelpersFileName =   "CurrentObjHelper"; // .bin is added
    
    // worldMap
    public static var GenerateMapNew = false;
    public static var ChanceForLuckySpot = 0.1; // chance that during generation an object is lucky and tons more of that are generated close by
    public static var startingGx = 235; // 235; //270; // 360;
    public static var startingGy = 150; //200;//- 400; // server map is saved y inverse 
    public static var CreateGreenBiomeDistance = 5;
   
    // food stuff
    public static var FoodUsePerSecond = 0.10; // 0.2; // 5 sec per pip // normal game has around 0.143 (7 sec) with bad temperature and 0.048 (21 sec) with good 
    public static var FoodReductionPerEating = 1;
    public static var MinAgeToEat = 3; // MinAgeToEat and MinAgeFor putting on cloths on their own
    public static var GrownUpFoodStoreMax = 15;
    public static var NewBornFoodStoreMax = 4;
    public static var OldAgeFoodStoreMax = 10;
    public static var DeathWithFoodStoreMax = 0; // Death through starvation if food store max reaches below XX 
    public static var IncreasedFoodNeedForChildren = 1.5; // children need XX food is below GrownUpAge
    public static var YumBonus = 3; // First time eaten you get XX yum boni, reduced one per eating. Food ist not yum after eating XX
    public static var YumFoodRestore = 0.8; // XX pipes are restored from a random eaten food. Zero are restored if random food is the current eaten food
    public static var YumNewCravingChance = 0.2; // XX chance that a new random craving is chosen even if there are existing ones

    // health
    public static var MinHealthPerYear = 1; // for calulating aging / speed: MinHealthPerYear * age is reduced from health(yum_mulpiplier)
    public static var HealthFactor = 20; // Changes how much health(yum_mulpiplier) affects speed and aging. (From 0.8 to 1.2)  (From 0.5 to 2) 
    // for example a men with age 20 needs 20 * MinHealthPerYear health(yum_mulpiplier) to be healthy
    //if(health >= 0) healthFactor = (1.2  * health + ServerSettings.HealthFactor) / (health + ServerSettings.HealthFactor);
    //else healthFactor = (health - ServerSettings.HealthFactor) / ( 2 * health - ServerSettings.HealthFactor);
    
    // starving to death
    public static var AgingFactorWhileStarvingToDeath = 0.5; // if starving to death aging is slowed factor XX up to GrownUpAge, otherwise aging is speed up factor XX
    public static var GrownUpAge = 14; // is used for AgingFactorWhileStarvingToDeath and for increase food need for children
    public static var StarvingToDeathMoveSpeedFactor = 0.5; // reduces speed if stored food is below 0
    public static var StarvingToDeathMoveSpeedFactorWhileHealthAboveZero = 0.8; // reduces speed if stored food is below 0 and health / yum multiplier > 0
    public static var FoodStoreMaxReductionWhileStarvingToDeath = 4; // (5) reduces food store max with factor XX for each food below 0

    public static var maxDistanceToBeConsideredAsClose = 200; //20; // only close players are updated with PU and MX and Movement 

    // for movement
    public static var InitialPlayerMoveSpeed:Float = 3.75; //3.75; // in Tiles per Second
    public static var SpeedFactor = 1; // MovementExtender // used to incease or deacrease speed factor X
    public static var MinSpeedReductionPerContainedObj = 0.98;
    
    public static var LetTheClientCheatLittleBitFactor = 1.1; // when considering if the position is reached, allow the client to cheat little bit, so there is no lag
    // TODO FIX this can make jumps if too small / ideally this should be 0 so that the client cannot cheat while moving
    public static var MaxMovementCheatingDistanceBeforeForce = 2; // if client player position is bigger then X, client is forced in PU to use server position 
    public static var ChanceThatAnimalsCanPassBlockingBiome = 0.05;

    // hungry work
    public static var HungryWorkCost = 10;

    // for animal movement
    public static var chancePreferredBiome = 0.8; // Chance that the animal ignors the chosen target if its not from his original biome
    
    // for animal offsprings
    public static var ChanceForOffspring = 0.001; // For each movement there is X chance to generate an offspring.   
    public static var ChanceForAnimalDying = 0.0005; // For each movement there is X chance that the animal dies  
    public static var MaxOffspringFactor = 1; // The population can only be at max X times the initial population

    public static var WorldTimeParts = 25; // in each tick 1/XX DoTimeSuff is done for 1/XX part of the map. Map height should be dividable by XX * 10 
    public static var ObjRespawnChance = 0.001; // 0.002; 17 hours // In each 20sec (WorldTimeParts/20 * 10) there is a X chance to generate a new object if number is less then original objects
    public static var ObjDecayChance = 0.0002; // 0.001; (X0.08)
    public static var ObjDecayFactorOnFloor:Float = 0.1;
    public static var ObjDecayFactorForFood:Float = 10;

    // AI
    public static var NumberOfAis:Int = 1;

    // combat
    //public static var WoundWhenFoodStoreMaxBelow = 10;


    // iron, tary spot spring cannot respawn or win lottery
    public static function CanObjectRespawn(obj:Int) : Bool
    {
        return (obj != 942 && obj !=3030 && obj != 2285 && obj != 3962 && obj != 503);
    }

    public static function PatchObjectData()
    {
        // allow some smithing on tables // TODO fix time transition for contained obj
        for(obj in ObjectData.importedObjectData)
        {
            if( obj.description.indexOf("on Flat Rock") != -1 )
            {
                obj.containSize = 2;
                obj.containable = true;
            }

            if( obj.description.indexOf("Well") != -1 || (obj.description.indexOf("Pump") != -1 && obj.description.indexOf("Pumpkin") == -1)
                || obj.description.indexOf("Vein") != -1 || obj.description.indexOf("Mine") != -1 || obj.description.indexOf("Iron Pit") != -1
                || obj.description.indexOf("Drilling") != -1 || obj.description.indexOf("Rig") != -1 || obj.description.indexOf("Ancient") != -1)
            {
                obj.decayFactor = -1;

                //trace('Settings: ${obj.description} ${obj.containSize}');
            }

            

            //if(obj.containable) trace('${obj.description} ${obj.containSize}');
        }

        // Change map spawn chances
        ObjectData.getObjectData(769).mapChance *= 5; // Wild Horse
        ObjectData.getObjectData(942).mapChance *= 10; // Muddy Iron Vein
        ObjectData.getObjectData(2135).mapChance /= 3; // Rubber Tree
        ObjectData.getObjectData(2156).mapChance *= 0.5; // Less UnHappy Mosquitos
        ObjectData.getObjectData(2156).biomes.push(BiomeTag.SWAMP); // Evil Mosquitos now also in Swamp
        
        
        ObjectData.getObjectData(418).biomes.push(BiomeTag.YELLOW); // Happy Wolfs now also in Yellow biome :)
        ObjectData.getObjectData(418).biomes.push(BiomeTag.GREEN); // Happy Wolfs now also in Green biome :)
        ObjectData.getObjectData(418).mapChance *= 1.5; // More Happy Wolfs
        ObjectData.getObjectData(418).speedMult = 1.5; // Boost Wolfs even more :)

        ObjectData.getObjectData(769).biomes.push(BiomeTag.GREEN); // Beautiful Horses now also in Green biome :)

        ObjectData.getObjectData(290).speedMult = 0.80; // Iron Ore
        ObjectData.getObjectData(314).speedMult = 0.80; // Wrought Iron
        ObjectData.getObjectData(326).speedMult = 0.80; // Steel Ingot
        ObjectData.getObjectData(838).mapChance = ObjectData.getObjectData(211).mapChance / 5; // Add some lovely mushrooms  
        ObjectData.getObjectData(838).biomes.push(BiomeTag.GREEN); // Add some lovely mushrooms 


        ObjectData.getObjectData(663).description += "+hungryWork"; // Deep Well // TODO test

         // nerve horse cart little bit :)
        ObjectData.getObjectData(778).speedMult = 1.50; // Horse-Drawn Cart
        ObjectData.getObjectData(3158).speedMult = 1.60; // Horse-Drawn Tire Cart
        
        ObjectData.getObjectData(484).speedMult = 0.85; // Hand Cart
        ObjectData.getObjectData(861).speedMult = 0.85; // // Old Hand Cart
        ObjectData.getObjectData(2172).speedMult = 0.9; // Hand Cart with Tires
        

        // nerve food
        //ObjectData.getObjectData(2143).foodValue = 6; // banana
        //ObjectData.getObjectData(31).foodValue = 4; // Gooseberry
        //ObjectData.getObjectData(2855).foodValue = 4; // Onion
        //ObjectData.getObjectData(808).foodValue = 4; // Wild Onion

        // boost hunted food
        ObjectData.getObjectData(197).foodValue = 18; // Cooked Rabbit 10 --> 18
        ObjectData.getObjectData(2190).foodValue = 24; // Turkey Slice on Plate 17 --> 24
        ObjectData.getObjectData(1285).foodValue = 18; // Omelette 12 --> 18

        ObjectData.getObjectData(518).useChance = 0.3; // Cooked Goose

         
        

        //trace('Insulation: ${ObjectData.getObjectData(128).getInsulation()}'); // Redd Skirt

        //ObjectData.getObjectData(31).writeToFile();

        //trace('Trace: ${ObjectData.getObjectData(8881).description}');
         
        
        
        //trace('Patch: ${ObjectData.getObjectData(942).description}');
        //if (obj.deadlyDistance > 0)
        //    obj.mapChance *= 0;  


    }

    public static function PatchTransitions(transtions:TransitionImporter)        
    {   
        for(trans in TransitionImporter.transitionImporter.transitions)
        {
            if(trans.actorID < -1) 
            {
                // trans.traceTransition("PatchTransitions: ", true);

                trans.actorID = 0; 
                transtions.addTransition("PatchTransitions: ", trans);
                trans.traceTransition("PatchTransitions: ");
            }

            if(trans.autoDecaySeconds == -168)
            {
                trans.autoDecaySeconds = -2; // use chance 20%: 10 uses / 120 min 0.08 Bowls per min

                //trans.traceTransition("PatchTransitions: ", true);    
            }

            if(trans.autoDecaySeconds == 9000) // 150 min like deep well 0.53 bowls per min
            {
                trans.autoDecaySeconds = 1200; // 20 min one bucket 12.5% (bucket has 10 uses): 80 uses / 20 min = 4 bowls per min

                //trans.traceTransition("PatchTransitions: ", true);    
            }

            if(trans.autoDecaySeconds == 2160) // 36 min like well  0.91 bowls per min
            {
                trans.autoDecaySeconds = 720; // 12 min one bowl 3%: 33 uses / 12 min = 2.7 bowls per min

                //trans.traceTransition("PatchTransitions: ", true);    
            }
        }

        // Original: Riding Horse: 770 + -1 = 0 + 1421
        var trans = new TransitionData(770,0,0,1421);
        transtions.addTransition("PatchTransitions: ", trans);

        // TODO this should function somehow with categories???
        // original transition makes cart loose rubber if putting down horse cart
        //Original: 3158 + -1 = 0 + 1422 // Horse-Drawn Tire Cart + ???  -->  Empty + Escaped Horse-Drawn Cart --> must be: 3158 + -1 = 0 + 3161
        trans = transtions.getTransition(3158, -1);
        trans.newTargetID = 3161;
        trans.traceTransition("PatchTransitions: ");

         // original transition makes cart loose rubber if picking up horse cart
        //Original:  0 + 3161 = 778 + 0 //Empty + Escaped Horse-Drawn Tire Cart# just released -->  Horse-Drawn Cart + Empty
        trans = transtions.getTransition(0, 3161);
        trans.newActorID = 3158;
        trans.traceTransition("PatchTransitions: ");

        trans = transtions.getTransition(-1, 3161);
        trans.newTargetID = 3157;
        trans.traceTransition("PatchTransitions: ");

        trans = transtions.getTransition(0, 3157);
        trans.newActorID = 3158;
        trans.traceTransition("PatchTransitions: ");


        // let Tule Stumps (122) grow back
        trans = transtions.getTransition(-1, 122); 
        trans.newTargetID = 121; // 121 = Tule Reeds
        trans.traceTransition("PatchTransitions: ");



        // 141 Canada Goose Pond
        //1261 Canada Goose Pond with Egg // TODO let egg come back
        //Server.transitionImporter.


        // change decay time for grave 88 = Grave
        //trans = transtions.getTransition(-1, 88); 
        //trans.autoDecaySeconds = 10; 
        //trans.traceTransition("PatchTransitions: ");

        // should be fixed now with the rest of the -2 transitions
        //-2 + 141 = 0 + 143 // some how we have -2 transactions like hand + ghoose pond = ghoose pond with feathers 
        //trans = transtions.getTransition(-2, 141); 
        //trans.actorID = 0; 
        //transtions.addTransition("PatchTransitions: ", trans);
        //trans.traceTransition("PatchTransitions: ");
        

        // let get berrys back!
        trans = new TransitionData(-1,30,0,30); // Wild Gooseberry Bush
        
        trans.reverseUseTarget = true;
        trans.autoDecaySeconds = 600;
        transtions.addTransition("PatchTransitions: ", trans);

        trans = new TransitionData(-1,279,0,30); // Empty Wild Gooseberry Bush --> // Wild Gooseberry Bush
        trans.reverseUseTarget = true; 
        trans.autoDecaySeconds = 600; 
        transtions.addTransition("PatchTransitions: ", trans);

        // let get bana back!
        trans = new TransitionData(-1,2142,0,2142); // Banana Plant
        trans.reverseUseTarget = true;
        trans.autoDecaySeconds = 1000;
        transtions.addTransition("PatchTransitions: ", trans);

        trans = new TransitionData(-1,2145,0,2142); // Empty Banana Plant --> Banana Plant
        trans.reverseUseTarget = true;
        trans.autoDecaySeconds = 1000; 
        transtions.addTransition("PatchTransitions: ", trans);

        //  Wild Gooseberry Bush
        trans = new TransitionData(253,30,253,30); // Bowl of Gooseberries + Wild Gooseberry Bush --> Bowl of Gooseberries(+1) + Wild Gooseberry Bush
        trans.reverseUseActor = true;
        transtions.addTransition("PatchTransitions: ", trans);

        trans = new TransitionData(235,30,253,30); // Clay Bowl + Wild Gooseberry Bush --> Bowl of Gooseberries + Wild Gooseberry Bush
        transtions.addTransition("PatchTransitions: ", trans);

        trans = new TransitionData(253,30,253,279); // Bowl of Gooseberries + Wild Gooseberry Bush (Last) --> Bowl of Gooseberries(+1) + Empty Wild Gooseberry Bush
        trans.reverseUseActor = true;
        transtions.addTransition("PatchTransitions: ", trans, false, true);

        trans = new TransitionData(235,30,253,279); // Clay Bowl + Wild Gooseberry Bush (Last) --> Bowl of Gooseberries + Empty Wild Gooseberry Bush
        transtions.addTransition("PatchTransitions: ", trans, false, true);


        // Domestic Gooseberry Bush
        trans = new TransitionData(253,391,253,391); // Bowl of Gooseberries + Domestic Gooseberry Bush --> Bowl of Gooseberries(+1) + Domestic Gooseberry Bush
        trans.reverseUseActor = true;
        transtions.addTransition("PatchTransitions: ", trans);

        trans = new TransitionData(235,391,253,391); // Clay Bowl + Domestic Gooseberry Bush --> Bowl of Gooseberries + Domestic Gooseberry Bush
        transtions.addTransition("PatchTransitions: ", trans);

        trans = new TransitionData(253,391,253,1135); // Bowl of Gooseberries + Domestic Gooseberry Bush (Last) --> Bowl of Gooseberries(+1) + Empty Domestic Wild Gooseberry Bush
        trans.reverseUseActor = true;
        transtions.addTransition("PatchTransitions: ", trans, false, true);

        trans = new TransitionData(235,391,253,1135); // Clay Bowl + Domestic Gooseberry Bush (Last) --> Bowl of Gooseberries + Empty Domestic Gooseberry  Bush
        transtions.addTransition("PatchTransitions: ", trans, false, true);


        // give wolfs some meat
        trans = transtions.getTransition(0, 423); // 423 Skinned Wolf 
        trans.newTargetID = 565;  // 565 Butchered Mouflon
        trans.traceTransition("PatchTransitions: "); 

        // allow to cook mutton on cloals
        trans = new TransitionData(569,85,570,85); // 569 Raw Mutton + 85 Hot Coals --> 570 Cooked Mutton + 85 Hot Coals
        transtions.addTransition("PatchTransitions: ", trans);

    }

    public static function writeToFile()
    {
        var rtti = haxe.rtti.Rtti.getRtti(ServerSettings);
        var dir = './${ServerSettings.SaveDirectory}/';
        var path = dir + "ServerSettings.txt";

        if(FileSystem.exists(dir) == false) FileSystem.createDirectory(dir);

        var writer = File.write(path, false);

        writer.writeString('Remove **default** if you dont want to use default value!\n');

        var count = 0;

        for (field in rtti.statics)
        {
            if('$field'.indexOf('CFunction') != -1 ) continue;
            count++;
            var value:Dynamic = Reflect.field(ServerSettings, field.name);

            //trace('ServerSettings: $count ${field.name} ${field.type} $value');
            
            if('${field.type}' == "CClass(String,[])")
            {
                writer.writeString('**default** ${field.name} = "$value"\n');
            }
            else
            {
                writer.writeString('**default** ${field.name} = $value\n');
            }
        }

        writer.writeString('**END**\n');

        writer.close();
    }

    public static function readFromFile(traceit:Bool = true) : Bool
    {
        var reader = null;

        try{
            //var rtti = haxe.rtti.Rtti.getRtti(ServerSettings);
            var dir = './${ServerSettings.SaveDirectory}/';
            reader = File.read(dir + "ServerSettings.txt", false);

            reader.readLine();

            var line = "";

            while(line.indexOf('**END**') == -1 )
            {
                line = reader.readLine();

                //trace('Read: ${line}');

                if(line.indexOf('**default**') != -1 ) continue;           

                var splitLine = line.split("=");                

                if(splitLine.length < 2) continue;

                splitLine[0] = StringTools.replace(splitLine[0], ' ', '');
                //splitLine[1] = StringTools.replace(splitLine[1], '\n', '');

                if(traceit) trace('Load Setting: ${splitLine[0]} = ${splitLine[1]}');

                var fieldName = splitLine[0];
                var value:Dynamic = splitLine[1];

                if(splitLine[1].indexOf('"') != -1 )
                {
                    var splitString = splitLine[1].split('"');

                    if(splitString.length < 3) continue;

                    value = splitString[1]; 
                }
                else 
                {
                    value = StringTools.replace(value, 'true', '1');
                    value = StringTools.replace(value, 'false', '0');
                    value = Std.parseFloat(value);
                }

                var oldValue:Dynamic = Reflect.field(ServerSettings, fieldName);

                Reflect.setField(ServerSettings, fieldName, value);

                var newValue:Dynamic = Reflect.field(ServerSettings, fieldName);

                if('$newValue' != '$oldValue') trace('Setting changed: ${fieldName} = ${newValue} // old value: $oldValue');
            }
        }
        catch(ex)
        {
            if(reader != null) reader.close();

            trace(ex);

            return false;
        }

        return true;

        //trace('Read Test: traceTransitionByTargetDescription: $traceTransitionByTargetDescription');
        //trace('Read Test: YumBonus: $YumBonus');
    }
}

/**Actor Category: 1641 @ Deadly Wolf
 Trans: 1640 + 0 = 427 + 1363 
Semi-tame Wolf# just fed 
 + Empty  
 --> Attacking Wolf
 + Bite Wound

Actor Category: 1641 @ Deadly Wolf
 Trans: 1630 + 0 = 427 + 1363 
 Semi-tame Wolf
 + Empty  
 --> Attacking Wolf
 + Bite Wound
 **/