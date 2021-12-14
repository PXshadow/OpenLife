package openlife.server;
import openlife.auto.AiHelper;
import openlife.server.Biome.BiomeTag;
import haxe.macro.Expr;
import openlife.macros.Macro;
import openlife.data.transition.TransitionImporter;
import openlife.auto.WorldInterface;
import openlife.auto.PlayerInterface;
import haxe.ds.BalancedTree;
import haxe.macro.Expr.Catch;
import haxe.display.Server.HaxeModuleMemoryResult;
import openlife.client.ClientTag;
import openlife.data.object.ObjectData;
import openlife.settings.ServerSettings;
import haxe.ds.Vector;
import openlife.data.object.ObjectHelper;
import openlife.data.map.MapData;
import openlife.data.transition.TransitionData;
import openlife.data.Pos;
import openlife.data.object.player.PlayerInstance;
import sys.thread.Mutex;

using StringTools;


using openlife.server.MoveHelper;

// GlobalPlayerInstance is used as a WorldInterface for an AI, since it may be limited what the AI can see so player information is relevant
class GlobalPlayerInstance extends PlayerInstance implements PlayerInterface implements WorldInterface
{
    // todo remove players once dead???
    public static var AllPlayers = new Map<Int,GlobalPlayerInstance>();
    public static function AddPlayer(player:GlobalPlayerInstance)
    {
        AllPlayers[player.p_id] = player;
        Lineage.AddLineage(player.p_id, player.lineage);
    }

    public static var lastAiEveOrAdam:GlobalPlayerInstance; 
    public static var lastHumanEveOrAdam:GlobalPlayerInstance; 
    public static var LastLeaderBadgeColor:Int = 0;

    public var lineage = new Lineage();

    // make sure to set these null is player is deleted so that garbage collector can clean up
    public var followPlayer:GlobalPlayerInstance;
    public var heldPlayer:GlobalPlayerInstance;
    public var heldByPlayer:GlobalPlayerInstance;

    // handles all the movement stuff
    public var moveHelper:MoveHelper;

    // additional ObjectInformation for the object stored in backpack or other clothing. The size is 6 since 6 clothing slots
    public var clothingObjects:Vector<ObjectHelper> = new Vector(6); 
    
    // is used since move and move update can change the player at the same time
    public var mutex = new Mutex();

    public var connection:Connection; 
    public var serverAi:ServerAi;

    public var trueAge:Float = ServerSettings.StartingEveAge;

    var hasEatenMap = new Map<Int, Float>();

    public var leaderBadgeColor:Int = LastLeaderBadgeColor++;

    // craving
    var currentlyCraving:Int = 0;
    var lastCravingIndex:Int = 0;
    var cravings = new Array<Int>();

    // combat 
    public var hits:Float = 0;
    public var woundedBy = 0;

    // birth stuff 
    public var childrenBirthMali:Float = 0;  // increases for each child // reduces for dead childs

    public var foodUsePerSecond = ServerSettings.FoodUsePerSecond; // is changed in update temperature

    public var exiledByPlayers = new Map<Int, GlobalPlayerInstance>();

    // set all stuff null so that nothing is hanging around
    public function delete()
    {
        this.followPlayer = null;

        this.heldPlayer = null;
        this.heldByPlayer = null;

        this.exiledByPlayers = null;
    }

    public var name(get, set):String;

    public function get_name()
    {
        return lineage.name;
    }

    public function set_name(newName:String)
    {
        return lineage.name = newName;
    }

    public var familyName(get, null):String;

    public function get_familyName()
    {
        return lineage.familyName;
    }

    public var mother(get, set):GlobalPlayerInstance;

    public function get_mother()
    {
        return lineage.mother;
    }

    public function set_mother(newMother:GlobalPlayerInstance)
    {
        return lineage.mother = newMother;
    }

    public var father(get, set):GlobalPlayerInstance;

    public function get_father()
    {
        return lineage.father;
    }

    public function set_father(newFather:GlobalPlayerInstance)
    {
        return lineage.father = newFather;
    }


    public static function GetNumberLifingPlayers() : Int
    {
        var numberLifingPlayers = 0;

        for (c in Connection.getConnections())
        {            
            if(c.player.deleted) continue;
            numberLifingPlayers++;
        }

        return numberLifingPlayers;
    }

    public function new(ai:ServerAi = null)
    {
        super([]);

        this.serverAi = ai;
        this.p_id = Server.server.playerIndex++;
        this.po_id = ObjectData.personObjectData[WorldMap.calculateRandomInt(ObjectData.personObjectData.length-1)].id;
        this.moveHelper = new MoveHelper(this);
        this.heldObject = ObjectHelper.readObjectHelper(this, [0]);        
        this.age_r = ServerSettings.AgingSecondsPerYear;

        AddPlayer(this);
        
        for(i in 0...this.clothingObjects.length)
        {
            this.clothingObjects[i] = ObjectHelper.readObjectHelper(this, [0]);
        }

        // TODO search most empty special biome for eve
        // TODO on big map dont spawn eve too far away
        // TODO less hostile environment for eve (since the plan is to make human free nature more dangerous)
        // TODO give a certain eve birth %

        // spawn human eve to human adam and ai eve to ai adam except if player count is very few 
        var isAi = this.isAi();
        var allowHumanSpawnToAIandAiToHuman = GetNumberLifingPlayers() <= ServerSettings.MaxPlayersBeforeStartingAsChild;
        var spawnEve = allowHumanSpawnToAIandAiToHuman || (isAi && lastAiEveOrAdam != null) || (isAi == false && lastHumanEveOrAdam != null);

        //if(false) spawnAsEve(allowHumanSpawnToAIandAiToHuman);
        if(spawnEve) spawnAsEve(allowHumanSpawnToAIandAiToHuman);
        else
        {
            if(spawnAsChild() == false) spawnAsEve(allowHumanSpawnToAIandAiToHuman);
        }        

        move_speed = MoveHelper.calculateSpeed(this, gx, gy);
        
        food_store_max = calculateFoodStoreMax();
        food_store = food_store_max / 2;
        yum_multiplier = ServerSettings.MinHealthPerYear * 3; // start with health for 3 years // TODO change

        for(c in Connection.getConnections())
        {
            c.send(ClientTag.NAME,['${this.p_id} ${this.name} ${this.familyName}']);
        }

        for(c in Connection.getConnections())
        {
            c.send(ClientTag.LINEAGE,[c.player.lineage.createLineageString()]);
        }

        Connection.SendFollowingToAll(this);
    
        // TODO inform AI about new player
    }

    private function spawnAsEve(allowHumanSpawnToAIandAiToHuman:Bool)
    {
        this.lineage.myEveId = this.p_id;

        var isAi = this.isAi();
        var lastEveOrAdam = isAi ? lastAiEveOrAdam : lastHumanEveOrAdam;

        if(allowHumanSpawnToAIandAiToHuman && lastEveOrAdam == null)
        {
            // try if the other one is not null
            lastEveOrAdam = isAi ? lastHumanEveOrAdam : lastAiEveOrAdam;
            lastAiEveOrAdam = null;
            lastHumanEveOrAdam = null;
        }

        age = ServerSettings.StartingEveAge;
        this.trueAge = ServerSettings.StartingEveAge;

        // TODO spawn eve in jungle with bananaplants
        gx = ServerSettings.startingGx;
        gy = ServerSettings.startingGy;

        if(lastEveOrAdam == null)
        {
            lastEveOrAdam = this;

            // give eve the right color fitting to closest special biome
            var closeSpecialBiomePersonColor = getCloseSpecialBiomePersonColor(this.tx(), this.ty());
            if(closeSpecialBiomePersonColor > 0)
            {
                var female = ServerSettings.ChanceForFemaleChild >= 0.5;
                var personsByColor = female ? ObjectData.femaleByRaceObjectData : ObjectData.maleByRaceObjectData;
                var persons = personsByColor[closeSpecialBiomePersonColor];
                po_id = persons[WorldMap.calculateRandomInt(persons.length-1)].id; 

                trace('Child is an EVE / ADAM with color: ${this.getColor()}');
            }
        }
        else
        {
            // Spawn An Eve / Adam is to last Eve / Adam
            this.followPlayer = lastEveOrAdam;
            //lastEveOrAdam.followPlayer = this;
            this.mother = lastEveOrAdam; // its not really the mother, but its the mother in spirit...  

            gx = lastEveOrAdam.tx();
            gy = lastEveOrAdam.ty();

            var female = lastEveOrAdam.isFemal() == false;
            var personsByColor = female ? ObjectData.femaleByRaceObjectData : ObjectData.maleByRaceObjectData;
            var persons = personsByColor[lastEveOrAdam.getColor()];
            po_id = persons[WorldMap.calculateRandomInt(persons.length-1)].id; 

            lastEveOrAdam = null;

            trace('An Eve / Adam is born to an Eve / Adam with color: ${this.getColor()}');
        }

        name = isFemal() ? "EVE" : "ADAM";

        if(isAi) lastAiEveOrAdam = lastEveOrAdam;
        else lastHumanEveOrAdam = lastEveOrAdam;
    } 


    // TODO higher change of children for smaler families
    // TODO spawn acording to prestiege score
    // TODO spawn in different classes (noble / citizen / worker)
    // TODO spawn noobs more likely noble
    // TODO spawn in hand of mother???
    // TODO dont spawn as child if far too much children
    private function spawnAsChild() : Bool
    {
        var mother:GlobalPlayerInstance = GetFittestMother(this.isAi());

        if(mother == null) return false;

        // TODO use childFood for birth and childfeeding
        // TODO father
        this.lineage.myEveId = mother.lineage.myEveId;
        this.mother = mother;
        this.followPlayer = mother; // the mother is the leader

        // TODO consider dead children for mother fitness

        mother.childrenBirthMali += 1; // make it less likely to get new child
        if(mother.mother != null) mother.mother.childrenBirthMali += 0.5; // make it less likely to get new child for each grandkid

        this.age = 0.01;
        this.trueAge = 0.01;
        gx = mother.tx();
        gy = mother.ty();

        
        var motherColor = mother.getColor();
        var color = motherColor;
        var female = ServerSettings.ChanceForFemaleChild > WorldMap.calculateRandomFloat(); 
        var personsByColor = female ? ObjectData.femaleByRaceObjectData : ObjectData.maleByRaceObjectData;
        var rand = WorldMap.calculateRandomFloat();
        var closeSpecialBiomePersonColor = getCloseSpecialBiomePersonColor(this.tx(), this.ty());
        var closeToWrongSpecialBiome = (closeSpecialBiomePersonColor > 0) && (motherColor != closeSpecialBiomePersonColor);
        var otherColorThenMom = closeToWrongSpecialBiome ? ServerSettings.ChanceForOtherChildColorIfCloseToWrongSpecialBiome > rand : ServerSettings.ChanceForOtherChildColor > rand;

        //trace('New child Rand: ${ServerSettings.ChanceForOtherChildColor} > $rand $otherColorThenMom '); 

        if(otherColorThenMom)
        {
            var colder;
            if(closeToWrongSpecialBiome)
            {
                colder = closeSpecialBiomePersonColor > motherColor; // lucky currently higher race Id means colder biome :)

                trace('Child: $colder closeSpecialBiomePersonColor: $closeSpecialBiomePersonColor > $motherColor');
            }
            else colder = WorldMap.calculateRandomFloat() > 0.5; 

            color = getCloseColor(motherColor, colder);

            trace('New child has other color then mother: motherColor: $motherColor color: $color colderbiome: $colder'); 
        }

        var persons = personsByColor[color];
        po_id = persons[WorldMap.calculateRandomInt(persons.length-1)].id; 
        
        trace('New child is born to mother: ${mother.name} ${mother.familyName} female: $female motherColor: $motherColor childColor: ${this.getColor()}');
        
        return true;
    } 

    // person ==> Ginger = 6 / White = 4 / Brown = 3 /  Black = 1  
    public function getColor() : Int
    {
        var obj = ObjectData.getObjectData(po_id);
        if(obj == null) return -1;

        return obj.person;
    }

    // returns a more close color. Ginger --> White --> Brown --> Black
    public static function getCloseColor(color:Int, colder:Bool) : Int
    {
        if(color == 6) return 4; // Ginger --> White
        if(color == 4) return colder ? 6 : 3; // White --> Ginger or Brown
        if(color == 3) return colder ? 4 : 1; // Brown --> White or Black
        if(color == 1) return 3; // Black --> Brown

        return -1;
    }
    // Snow --> Ginger / Swamp --> White / Jungle --> Brown / Desert --> Black
    public static function getCloseSpecialBiomePersonColor(x:Int, y:Int) : Int
    {
        var maxSearch = 200;
        var biome = -1;
        var personColorByBiome = -1;
        var ii = 0;

        for(ii in 0...maxSearch)
        {
            // diagonal search
            biome = WorldMap.world.getBiomeId(x + ii, y + ii);
            personColorByBiome = PersonColor.getPersonColorByBiome(biome);
            if(personColorByBiome > 0) break;

            biome = WorldMap.world.getBiomeId(x - ii, y + ii);
            personColorByBiome = PersonColor.getPersonColorByBiome(biome);
            if(personColorByBiome > 0) break;

            biome = WorldMap.world.getBiomeId(x + ii, y - ii);
            personColorByBiome = PersonColor.getPersonColorByBiome(biome);
            if(personColorByBiome > 0) break;

            biome = WorldMap.world.getBiomeId(x - ii, y - ii);
            personColorByBiome = PersonColor.getPersonColorByBiome(biome);
            if(personColorByBiome > 0) break;

            // cross search
            biome = WorldMap.world.getBiomeId(x + ii, y);
            personColorByBiome = PersonColor.getPersonColorByBiome(biome);
            if(personColorByBiome > 0) break;

            biome = WorldMap.world.getBiomeId(x - ii, y);
            personColorByBiome = PersonColor.getPersonColorByBiome(biome);
            if(personColorByBiome > 0) break;

            biome = WorldMap.world.getBiomeId(x, y + ii);
            personColorByBiome = PersonColor.getPersonColorByBiome(biome);
            if(personColorByBiome > 0) break;

            biome = WorldMap.world.getBiomeId(x, y - ii);
            personColorByBiome = PersonColor.getPersonColorByBiome(biome);
            if(personColorByBiome > 0) break;
        }

        trace('Child: closeSpecialBiome: $biome personColor: $personColorByBiome distance: $ii');

        return personColorByBiome;
    }

    private static function GetFittestMother(childIsHuman:Bool) : GlobalPlayerInstance
    {
        var mother:GlobalPlayerInstance = null;
        var fitness = -1000.0;

        // search fertile mother
        for (c in Connection.getConnections())
        {            
            var tmpFitness = CalculateMotherFitness(c.player);

            if(childIsHuman == false) tmpFitness += ServerSettings.HumanMotherBirthMaliForAiChild;

            trace('Child: Fitness player mother: $tmpFitness ${c.player.name} ${c.player.familyName}');

            if(tmpFitness < -100) continue;

            if(tmpFitness > fitness || mother == null)
            {
                mother = c.player;
                fitness = tmpFitness;    
            }
        }

        // search fertile mother
        for (ai in Connection.getAis())
        {           
            var tmpFitness = CalculateMotherFitness(ai.player);

            if(childIsHuman) tmpFitness += ServerSettings.AiMotherBirthMaliForHumanChild;

            trace('Child: Fitness AI mother: $tmpFitness ${ai.player.name} ${ai.player.familyName}');

            if(tmpFitness < -100) continue;

            if(tmpFitness > fitness || mother == null)
            {
                mother = ai.player;
                fitness = tmpFitness;    
            }
        }

        return mother;
    }

    private static function CalculateMotherFitness(p:GlobalPlayerInstance) : Float
    {
        if(p.deleted) return -1000;
        if(p.isFertile() == false) return -1000;
        if(p.food_store < 3) return -100; // no starving mothers

        var tmpFitness = p.childrenBirthMali * (-1); // the more children the less likely
        tmpFitness += p.food_store /= 10; // the more food the more likely 
        tmpFitness += p.food_store_max /= 10; // the more healthy the more likely 
        tmpFitness += p.yum_bonus /= 20; // the more yum / prestige the more likely 
        var temperatureMail = Math.pow(((p.heat - 0.5) * 10), 2) / 10; // between 0 and 2.5 for very bad temperature
        tmpFitness -= temperatureMail;

        if(p.heldObject.objectData.speedMult > 1.1) tmpFitness -= 2; // if player is using fast objects
        else if(p.heldObject.id != 0) tmpFitness -= 1; // if player is holding objects
        
        return tmpFitness;
    }

    public function getPlayerInstance() : PlayerInstance
    {
        return this;
    }

    public function getWorld() : WorldInterface
    {
        return this;
    } 

    public function isMoving()
    {
        return moveHelper.isMoveing();      
    }

    public function getObjectData(id:Int) : ObjectData
    {
        return ObjectData.getObjectData(id);
    }

    //** faster way of getting ObjectData wihout needing to create the object first. Use this instead getObjectHelper if you just want the ObjectData **/
    public function getObjectDataAtPosition(x:Int, y:Int) : ObjectData
    {
        return WorldMap.world.getObjectDataAtPosition(x, y);
    }

    public function getTrans(actor:ObjectHelper, target:ObjectHelper) : TransitionData
    {
        return TransitionImporter.GetTrans(actor, target);
    }

    public function getTransition(actorId:Int, targetId:Int, lastUseActor:Bool = false, lastUseTarget:Bool = false, maxUseTarget:Bool=false):TransitionData
    {
        return TransitionImporter.GetTransition(actorId, targetId, lastUseActor, lastUseTarget, maxUseTarget);
    }

    public function getTransitionByNewTarget(newTargetId:Int) : Array<TransitionData>
    {
        return TransitionImporter.GetTransitionByNewTarget(newTargetId);
    }

    public function getTransitionByNewActor(newActorId:Int) : Array<TransitionData>
    {
        return TransitionImporter.GetTransitionByNewActor(newActorId);
    }

    public function getBiomeId(x:Int, y:Int):Int
    {
        return WorldMap.world.getBiomeId(x,y);
    }

    public function isBiomeBlocking(x:Int, y:Int) : Bool
    {
        return WorldMap.isBiomeBlocking(x,y);
    }
    
    //** returns NULL of x,y is too far away from player **/
    public function getObjectId(x:Int, y:Int):Array<Int>
    {
        // TODO check if too far away
        return WorldMap.world.getObjectId(x,y);
    }

    //** returns NULL of x,y is too far away from player / allowNull means it wont create a object helper if there is none **/
    public function getObjectHelper(x:Int, y:Int, allowNull:Bool = false) : ObjectHelper
    {
        // TODO check if too far away
        return WorldMap.world.getObjectHelper(x,y, allowNull);
    }

    //** returns -1 of x,y is too far away from player **/
    public function getFloorId(x:Int, y:Int):Int
    {
        // TODO check if too far away
        return WorldMap.world.getFloorId(x,y);
    }

    public function doEmote(id:Int, seconds:Int = -10)
    {
        this.connection.emote(id);
    }

    public function remove(x:Int, y:Int, index:Int = -1) : Bool
    {
        return TransitionHelper.doCommand(this, ServerTag.REMV, x, y, index);
    }

    public function drop(x:Int, y:Int, clothingIndex:Int=-1) : Bool
    {
        if(heldPlayer == null)
        {
            return TransitionHelper.doCommand(this, ServerTag.DROP, x, y, clothingIndex);
        }
        else{
            return dropPlayer();
        }
    }

    // TODO better use relative toData which transforms x,y to relative position
    public function toRelativeData(forPlayer:PlayerInstance):String
    {
        var heldObject = o_id[0] < 0 ?  '${o_id[0]}' : MapData.stringID(o_id);

        return toData(
            tx() - forPlayer.gx,
            ty() - forPlayer.gy,
            Std.int(age * 100) / 100, Std.int(age_r * 100) / 100,
            Std.int(move_speed * 100) / 100,
             heldObject,
             this.gx - forPlayer.gx,
             this.gy - forPlayer.gy
        );
    }

    /*
    USE x y id i#

    USE  is for bare-hand or held-object action on target object in non-empty 
     grid square, including picking something up (if there's no bare-handed 
     action), and picking up a container.
     id parameter is optional, and is used by server to differentiate 
     intentional use-on-bare-ground actions from use actions (in case
     where target animal moved out of the way).
     i parameter is optional, and specifies a container slot to use a held
     object on (for example, using a knife to slice bread sitting on a table).
    */
    public function use(x:Int, y:Int, containerIndex:Int = -1, target:Int = 0) : Bool
    {
        return TransitionHelper.doCommand(this, ServerTag.USE, x, y, containerIndex, target);
    }    

    public function isCloseToPlayer(player:GlobalPlayerInstance, distance:Int = 1)
    {
        var targetX = player.tx() - this.gx;
        var targetY = player.ty() - this.gy;

        return isClose(targetX,targetY,distance);
    }

    /** works with coordinates relative to the player **/ //TODO does not consider round map
    public function isClose(x:Int, y:Int, distance:Int = 1):Bool
    {    
        return (((this.x - x) * (this.x - x) <= distance * distance) && ((this.y - y) * (this.y - y) <= distance * distance));
    }

    public function getPackpack() : ObjectHelper
    {
        return this.clothingObjects[5];
    }

    public function hasBothShoes() : Bool
    {
        if (this.clothingObjects[2] == null || this.clothingObjects[3] == null)
            return false;
        return (this.clothingObjects[2].id != 0 && this.clothingObjects[3].id != 0) ;   
    }

    public function addFood(foodValue:Float)
    {
        this.food_store += foodValue;

        if (food_store > food_store_max)
        {
            this.yum_bonus = food_store - food_store_max;
            food_store = food_store_max;
        } 
    }

    public function CalculateHealthFactor(forSpeed:Bool) : Float
    {
        var health:Float = this.yum_multiplier; // - this.trueAge  * ServerSettings.MinHealthPerYear;

        var healthFactor:Float; 

        var maxBoni = forSpeed ? 1.2 : 2; // for Speed or for aging
        var maxMali = forSpeed ? 0.8 : 0.5;

        if(health >= 0) healthFactor = (maxBoni  * health + ServerSettings.HealthFactor) / (health + ServerSettings.HealthFactor);
        else healthFactor = (health - ServerSettings.HealthFactor) / ( (1 / maxMali) * health - ServerSettings.HealthFactor);

        return healthFactor;
    }



    /**
    PS
    p_id/isCurse text
    p_id/isCurse text
    p_id/isCurse text
    ...
    p_id/isCurse text
    #

    isCurse is 0 if not a successful curse, or 1 if successful.

    Text that each player says must not contain # or newline.

    Example:

    PS
    1432/0 HELLO THERE
    1501/1 CURSE JOHN SMITH
    1448/0 HELP ME
    #
    **/
    public function say(text:String)
    {
        this.mutex.acquire();

        //trace('say: $text');

        try
        {
            var player = this;
            var curse = 0;
            var id = player.p_id;

            text = text.toUpperCase();

            if(StringTools.contains(text, '!'))
            {
                if(ServerSettings.AllowDebugCommmands) DoDebugCommands(player, text);
            }

            
            
            var maxLenght = player.age < 10 ? Math.ceil(player.age * 2) : player.age < 20 ? Math.ceil(player.age * 4) : 80; 

            if(text.startsWith('/') == false &&  text.length > maxLenght) text = text.substr(0, maxLenght);

            doNaming(text);

            doCommands(text);

            for (c in Connection.getConnections())
            {
                c.send(PLAYER_SAYS,['$id/$curse $text']);
                c.send(FRAME);
            }

            for (ai in Connection.getAis())
            {
                ai.say(player,curse == 1,text);
            }
            
        }
        catch(ex)
        {
            trace(ex.details);
        }

        this.mutex.release();
    }

    private function doCommands(message:String)
    {
        var name = GetName(message);

        var doCommand = message.startsWith('I EXILE ');

        if(doCommand)
        {
            var target = GetPlayerByName(name);
            
            if(target == null || target == this) return;
            if(target.exiledByPlayers.exists(target.p_id)) return; // cannot exile twice before redeemed

            target.exiledByPlayers[target.p_id] = target;

            this.connection.sendGlobalMessage('YOU_EXILED:_${target.name}_${target.familyName}');
            target.connection.sendGlobalMessage('YOU_HAVE_BEEN_EXILED_BY:_${this.name}_${this.familyName}');

            Connection.SendExileToAll(this, target);

            this.doEmote(Emote.angry);
        }

        var doFollow = message.startsWith('I FOLLOW ');

        if(doFollow)
        {
            if(name == "ME")
            {
                // TODO check if follower color changes to new color or if needed to be send again

                this.followPlayer = null;

                this.connection.sendGlobalMessage('YOU_FOLLOW_NOW_NO_ONE!');

                Connection.SendFollowingToAll(this);

                this.doEmote(Emote.happy);

                return;
            }

            var player = GetPlayerByName(name);
            
            if(player == null || player == this.followPlayer) return;

            // TODO allow other leader through follow?

            var tmpFollow = this.followPlayer;
            this.followPlayer = player;
            var leader = this.getTopLeader();

            if(leader == null)
            {
                trace('FOLLOW: CIRCULAR FOLLOW --> NO CHANGE');
                this.followPlayer = tmpFollow;
                return;
            }

            this.connection.sendGlobalMessage('YOU_FOLLOW_NOW:_${player.name}_${player.familyName}');

            Connection.SendFollowingToAll(this);

            // inform leader
            if(leader.connection != null) leader.connection.sendMapLocation(leader, 'FOLLOWER', 'follower');
            if(leader.connection != null) leader.connection.sendGlobalMessage('YOU_HAVE_A_NEW_FOLLOWER:_${this.name}_${this.familyName}');

            this.doEmote(Emote.happy);
            
            return;
        }
    }

    // TODO support family name???
    public function GetPlayerByName(name:String) : GlobalPlayerInstance
    {
        //trace('Get Player name: $name');

        if(name.length < 3) return null;
        if(name == "YOU") return this.getClosestPlayer(6); // 6

        var bestPlayer = null;
        var bestDistance:Float = 10000; // 100 tiles
            
        for(p in AllPlayers)
        {
            //trace('Get Player p name: ${p.name}');

            if(p.name == name)
            {
                var distance = AiHelper.CalculateDistanceToPlayer(this, p);
                
                if(distance < bestDistance)
                {
                    bestPlayer = p;
                    bestDistance = distance; 
                }
            }
        }

        //if(bestPlayer != null) trace('Get Player: Found name: $name is ${bestPlayer.name} ${bestPlayer.familyName}');

        return bestPlayer;
    }

    public static function GetName(text:String) : String
    {
        var strings = text.split(' ');

        if(strings.length < 3) return "";
        
        var name = strings[2];

        return name;
    }

    // if people follow circular outcome is null / max 10 deep hierarchy is supported
    public function getTopLeader() : GlobalPlayerInstance
    {
        var leader = followPlayer;
        if(leader == null) return null;

        for(ii in 0...10)
        {
            if(leader.followPlayer == null) return leader;

            leader = leader.followPlayer;
        }

        return null;
    }

    /*
    NM
    p_id first_name last_name
    p_id first_name last_name
    p_id first_name last_name
    ...
    p_id first_name last_name
    #


    Gives name of player p_id.

    last_name may be ommitted.
    */
    public function doNaming(text:String)        
    {
        //trace('TEST Naming1: $text');

        var doFamilyName = text.startsWith('I AM');
        
        if(doFamilyName == false && text.startsWith('YOU ARE') == false) return;

        var player = doFamilyName ? this : this.heldPlayer;
        
        if(player == null) player = this.getClosestPlayer(5); // 5

        //trace('TEST Naming2: $text');

        if(player == null) return;

        if(doFamilyName)
        {
            if(player.familyName != ServerSettings.StartingFamilyName) return;
        }
        else if(player.name != ServerSettings.StartingName) return;

        var strings = text.split(' ');

        if(strings.length < 3) return;
        
        var name = strings[2];

        if(name.length < 3) return;

        //var r = ~/^[a-z]+$/i; // only letters
        var r = ~/[^a-z]/i; // true if anything but letters
        if(r.match(name)) return; // return if there is anything but letters

        // TODO choose name from list
        
        trace('TEST Naming: $name');

        

        if(doFamilyName)
        {
            player.lineage.setFamilyName(name);

            // TODO use family name from family head
        }
        else
        {
            // check if name is used
            for(c in Connection.getConnections())
            {
                if(c.player.name == name && c.player.familyName == this.familyName)
                {
                    trace('name: "$name" is used already!');

                    return;
                }
            }

            for(ai in Connection.getAis())
            {
                if(ai.player.name == name && ai.player.familyName == this.familyName)
                {
                    trace('name: "$name" is used already!');

                    return;
                }
            }

            player.name = name;
        }

        trace('TEST Naming: ${player.p_id} ${player.name} ${player.familyName}');
       

        for(c in Connection.getConnections())
        {
            c.send(ClientTag.NAME,['${player.p_id} ${player.name} ${player.familyName}']);
        }
    }

    /*public function eat() {
        return self();
    }*/

    /*
    SELF x y i#

    SELF is special case of USE action taken on self (to eat what we're holding
     or add/remove clothing).
     This differentiates between use actions on the object at our feet
     (same grid cell as us) and actions on ourself.
     If holding food i is ignored.
	 If not holding food, then SELF removes clothing, and i specifies
	 clothing slot:
     0=hat, 1=tunic, 2=frontShoe, 3=backShoe, 4=bottom, 5=backpack
    */
    public function self(x:Int = 0, y:Int = 0, clothingSlot:Int = -1)
    {
        var done = false;

        this.mutex.acquire();

        if(ServerSettings.debug)
        {
            done = doSelf(x,y,clothingSlot);
        }
        else{
            try
            {
                done = doSelf(x,y,clothingSlot);
            }
            catch(e)
            {                
                trace(e);
            }
        }

        // send always PU so that player wont get stuck
        if(done == false)
        {
            this.connection.send(PLAYER_UPDATE,[this.toData()]);
            this.connection.send(FRAME);
        }

        this.mutex.release();
    }
    
    public function move(x:Int,y:Int,seq:Int, moves:Array<Pos>)
    {
        MoveHelper.move(this, x, y, seq, moves);
    }

    private function doSelf(x:Int, y:Int, clothingSlot:Int) : Bool
    {
        trace('self: ${this.o_id[0]} ${heldObject.objectData.description} clothingSlot: $clothingSlot');

        if(clothingSlot < 0)
        {
            if(doEating(this,this)) return true;
        }

        if(doSwitchCloths(this, this, clothingSlot)) return true;

        return doPlaceObjInClothing(clothingSlot);
    }

    //UBABY x y i id#
    /*
    UBABY is a special case of SELF applied to a baby (to feed baby food
	  or add/remove clothing from baby).  Also works on elderly.
      Note that server currently allows UBABY to feed anyone food, but
      only putting clothing on babies and elderly.
      ALSO:  UBABY is used for healing wounded players.
      Essentially, any action where held item is used on another player.
      Should be called UOTHER, but UBABY is used for historical reasons.
      NOTE the alternate call for UBABY with extra id parameter.
      this specifies a specific person to do the action on, if more than one is
	  close to the target tile.
    */
    public function doOnOther(x:Int, y:Int, clothingSlot:Int, playerId:Int) : Bool
    {
        var targetPlayer = getPlayerAt(x,y, playerId);

        var done = false;

        if(targetPlayer == null)
        {
            this.connection.send(PLAYER_UPDATE,[this.toData()]);

            trace('doOnOtherHelper: could not find target player!');

            return false;
        }

        this.mutex.acquire();

        // make sure that if both players at the same time try to interact with each other it does not end up in a dead lock 
        while(targetPlayer.mutex.tryAcquire() == false)
        {
            this.mutex.release();

            Sys.sleep(WorldMap.calculateRandomFloat() / 5);

            this.mutex.acquire();
        }        
        
        if(ServerSettings.debug)
        {
            done = doOnOtherHelper(x,y,clothingSlot, targetPlayer);
        }
        else
        {
            try
            {
                done = doOnOtherHelper(x,y,clothingSlot, targetPlayer);
            }
            catch(e)
            {                
                trace(e);
            }
        }

        // send always PU so that player wont get stuck
        if(done == false)
        {
            this.connection.send(PLAYER_UPDATE,[this.toData()]);
            this.connection.send(FRAME);
        }

        if(targetPlayer != null) targetPlayer.mutex.release();
        this.mutex.release();

        return done;
    }

    public function doOnOtherHelper(x:Int, y:Int, clothingSlot:Int, targetPlayer:GlobalPlayerInstance) : Bool
    {
        trace('doOnOtherHelper: playerId: ${targetPlayer.p_id} ${this.o_id[0]} ${heldObject.objectData.description} clothingSlot: $clothingSlot');

        // 838 Dont feed dam drugs! Wormless Soil Pit with Mushroom // 837 Psilocybe Mushroom
        if(heldObject.objectData.isDrugs()) return false;        

        if(this.isClose(targetPlayer.tx() - this.gx , targetPlayer.ty() - this.gy) == false)
        {
            trace('doOnOtherHelper: Targt position is too far away player: ${this.tx()},${this.ty()} target: ${targetPlayer.tx},${targetPlayer.ty}');
            return false; 
        }

        if(clothingSlot < 0)
        {
            if(doEating(this, targetPlayer)) return true;
        }

        if(doSwitchCloths(this, targetPlayer, clothingSlot)) return true;

        return false;
    }

    public function getPlayerAt(x:Int, y:Int, playerId:Int) : GlobalPlayerInstance
    {
        return Connection.getPlayerAt(x,y,playerId);
    }

    public function getClosestPlayer(maxDistance:Int, onlyHuman:Bool = false) : GlobalPlayerInstance
    {
        // TODO limit max distance for ai

        var player:GlobalPlayerInstance = null;
        var distance = maxDistance * maxDistance;

        for(c in Connection.getConnections())
        {
            if(c.player.deleted) continue;

            if(c.player == this) continue;

            var pX = c.player.tx() - this.gx;
            var pY = c.player.ty() - this.gy;
            var tmpDistance = (pX - x) * (pX - x) + (pY - y) * (pY - y);

            if(tmpDistance < distance) player = c.player;
        }

        if(onlyHuman) return player;

        for(ai in Connection.getAis())
        {
            if(ai.player.deleted) continue;

            if(ai.player == this) continue;
            
            var pX = ai.player.tx() - this.gx;
            var pY = ai.player.ty() - this.gy;
            var tmpDistance = (pX - x) * (pX - x) + (pY - y) * (pY - y);

            if(tmpDistance < distance) player = ai.player;
        }

        return player;
    }

    

    /*
        FX

        food_store food_capacity last_ate_id last_ate_fill_max move_speed responsible_id
        yum_bonus yum_multiplier#

        food_store is integer amount of food left in body, capacity is the integer 
        maximum amount of food.

        last_ate_id is the object id of the last piece of food eaten, or 0 if nothing
        was eaten recently

        last_ate_fill_max is an integer number indicating how many slots were full
        before what was just eaten.  Amount that what was eaten filled us up is
        (food_store - last_ate_fill_max).

        move_speed is floating point speed in grid square widths per second.

        responsible_id is id of player that fed you if you're a baby, or -1

        yum_bonus is an integer indicating the current stored bonus food.

        yum_multiplier is an integer indicating how many yum bonus points are earned
        when the next yummy food is eaten.
    */

    public function sendFoodUpdate(isPlayerAction:Bool = true)
    {
        if(connection == null) return;

        //trace('\n\tFX food_store: ${Math.ceil(food_store)} food_capacity: ${Std.int(food_capacity)} last_ate_id: $last_ate_id last_ate_fill_max: $last_ate_fill_max move_speed: $move_speed responsible_id: $responsible_id yum_bonus: $yum_bonus yum_multiplier: $yum_multiplier');
        var cut_move_speed = Std.int(move_speed * 100) / 100;

        this.connection.send(FOOD_CHANGE,['${Math.ceil(food_store)} ${Std.int(food_store_max)} $last_ate_id $last_ate_fill_max $cut_move_speed $responsible_id ${Math.ceil(yum_bonus)} ${Math.ceil(yum_multiplier)}'], isPlayerAction);
    }

    public static function doEating(playerFrom:GlobalPlayerInstance, playerTo:GlobalPlayerInstance) : Bool
    {
        if (playerFrom.o_id[0] == 0) return false;

        if(playerFrom.age < ServerSettings.MinAgeToEat)
        {
            trace('too young to eat player.age: ${playerFrom.age} < ServerSettings.MinAgeToEat: ${ServerSettings.MinAgeToEat} ');
            return false;
        }

        var heldObjData = playerFrom.heldObject.objectData;
        if(heldObjData.dummyParent != null) heldObjData = heldObjData.dummyParent;

        var foodValue:Float = heldObjData.foodValue;

        trace('FOOD: food_store_max: ${playerTo.food_store_max} food_store: ${playerTo.food_store} foodValue: ${foodValue}');

        if(foodValue < 1)
        {
            trace('cannot eat this stuff no food value!!! ${heldObjData.description}');
            return false;
        }
        
        if(playerTo.food_store_max - playerTo.food_store < Math.ceil(foodValue / 3))
        {
            trace('too full to eat: food_store_max: ${playerTo.food_store_max} - food_store: ${playerTo.food_store} < foodValue: $foodValue  / 3');
            playerTo.doEmote(Emote.refuseFood);
            return false;
        }

        var countEaten = playerTo.hasEatenMap[heldObjData.id]; 
        if(countEaten < 0) countEaten = 0;    

        foodValue += ServerSettings.YumBonus;
        foodValue -= countEaten;

        var isCravingEatenObject = heldObjData.id == playerTo.currentlyCraving;
        if(isCravingEatenObject) foodValue += 1; // craved food give more boni

        var isSuperMeh = foodValue < playerFrom.heldObject.objectData.foodValue / 2;

        if(isSuperMeh) foodValue = playerFrom.heldObject.objectData.foodValue / 2;

        /*
        if(isSuperMeh && food_store > 0)
        {
            trace('when food value is less then halve it can only be eaten if starving to death: foodValue: $foodValue original food value: ${heldObject.objectData.foodValue} food_store: $food_store');
            return;
        }*/

        var isHoldingYum = playerFrom.isHoldingYum();

        if(isSuperMeh == false)
        {
            playerTo.hasEatenMap[heldObjData.id] += ServerSettings.FoodReductionPerEating;
            playerTo.doIncreaseFoodValue(heldObjData.id);
        }

        // eating YUM increases prestige / score while eating MEH reduces it
        if(isHoldingYum)
        {
            if(isCravingEatenObject)
            {
                playerTo.yum_multiplier += 2;
                if(playerFrom != null) playerFrom.yum_multiplier += 0.5;
            }
            else playerTo.yum_multiplier += 1;            
        }
        else playerTo.yum_multiplier -= 1;
             
        //else if(isHoldingMeh()) yum_multiplier -= 1;

        trace('YUM: ${heldObjData.description} foodValue: $foodValue countEaten: $countEaten');

        // food_store food_capacity last_ate_id last_ate_fill_max move_speed responsible_id
       
        playerTo.last_ate_fill_max = Math.ceil(playerTo.food_store);
        trace('last_ate_fill_max: ${playerTo.last_ate_fill_max}');
        //this.food_store += foodValue;
        playerTo.just_ate = 1;
        playerTo.last_ate_id = heldObjData.id;
        playerTo.responsible_id = playerFrom.p_id; // -1; // self???
        //this.o_transition_source_id = -1;

        playerTo.addFood(foodValue);

        playerTo.move_speed = MoveHelper.calculateSpeed(playerTo, playerTo.tx(), playerTo.ty());

        playerTo.sendFoodUpdate();

        // check if there is a player transition like:
        // 2143 + -1 = 2144 + 0 Banana
        // 1251 + -1 = 1251 + 0 lastUseActor: false Bowl of Stew
        // 1251 + -1 = 235 + 0 lastUseActor: true Bowl of Stew
        if(TransitionHelper.DoChangeNumberOfUsesOnActor(playerFrom.heldObject, false, false) == false)
        {
            trace('FOOD: set held object null');
            playerFrom.setHeldObject(null);
        }

        playerTo.SetTransitionData(playerTo.x, playerTo.y, true); // TODO needs to be set right

        Connection.SendUpdateToAllClosePlayers(playerTo);

        if(playerFrom != playerTo)
        {
            playerFrom.SetTransitionData(playerTo.x, playerTo.y, true); // TODO needs to be set right

            Connection.SendUpdateToAllClosePlayers(playerFrom);
        }

        playerTo.just_ate = 0;
        playerTo.action = 0;

        if(isCravingEatenObject)
        {
            playerTo.doEmote(Emote.miamFood);
            if(playerFrom != null) playerFrom.doEmote(Emote.happy);
        }
        else if(isHoldingYum) playerTo.doEmote(Emote.happy);
        else if(isSuperMeh) playerTo.doEmote(Emote.ill);
        else playerTo.doEmote(Emote.hmph);
        
        return true;    
    }

    /**
        PU
        List of player ids with their display object ids, facing direction, action
        attempt flag, action attempt target position,
        held object ids (in CONTAINER OBJECT FORMAT, see above), 
        whether held origin is valid (1 or 0), origin position on map of that held 
        object (where it was picked up from), 
        transition source object id (or -1) if held object is result of a transition,
        player's current heat value, 
        done_moving_seqNum (to signal destination reached), force flag (to signal
        a move truncated unexpectedly), x,y grid positions of player,
        floating point age in "years", floating point aging rate in sec/year (how many
        seconds it takes the player to age 1 year), and
        floating point move speeds (in grid square widths per second) and clothing
        set, just_ate = 1 or 0 to indicate whether the player just ate what they were 
        holding, the ID of the object they just ate, and the player responsible for this update.

        If facing is 0, then the player's facing direction doesn't change.
        If facing is 1, then they should face right, and -1 to face left.

        action flag is 1 if player is attempting an action, 0 otherwise;

        Heat is the player's warmth between 0 and 1, where 0 is coldest, 1 is hottest,
        and 0.5 is ideal.

        If done_moving_seqNum is > 0, this means the player is stationary at this position (and this is the sequence number of their last move).
        Otherwise, player may still be in the middle of a move (for example, if what
        they are holding decays while they are moving, a PU will be sent with
        done_moving_seqNum set to 0).

        force is usually 0 except in special cases of move truncation where it is 1.
        A player receiving force for itself must snap back to that location
        before continuing to move.
    **/
    public function SetTransitionData(x:Int, y:Int, objOriginValid = false)
    {
        var player = this;

        player.forced = false;
        player.action = 1;        
        player.o_id = this.heldPlayer != null ? this.o_id = [-heldPlayer.p_id] : this.heldObject.toArray();

        //player.o_transition_source_id = this.newTransitionSource; TODO ??????????????????????????
        player.o_transition_source_id = objOriginValid ? -1 : this.heldObject.id;

        // this changes where the client moves the object from on display
        player.o_origin_x = objOriginValid ? x : 0;
        player.o_origin_y = objOriginValid ? y : 0;
        
        player.o_origin_valid = objOriginValid ? 1 : 0; // if set to 0 no animation is displayed to pick up hold obj from o_origin_x o_origin_y
        
        player.action_target_x = x;
        player.action_target_y = y;
    }

    /*
    CR
    food_id bonus
    #

    Tells player about which food they're currently craving, and how much their
    YUM multiplier will increase when they eat it.
    */

    private function doIncreaseFoodValue(eatenFoodId:Int)
    {
        trace('IncreaseFoodValue: ${eatenFoodId}');
        
        if(hasEatenMap[eatenFoodId] > 0) cravings.remove(eatenFoodId);

        var hasEatenKeys = [for(key in hasEatenMap.keys()) key];

        trace('IncreaseFoodValue: hasEatenKeys.length: ${hasEatenKeys.length}');

        // restore one food pip if eaten not super meh
        if(hasEatenKeys.length < 1) return;

        var random = WorldMap.calculateRandomInt(hasEatenKeys.length -1);
        var key = hasEatenKeys[random];

        //trace('IncreaseFoodValue: random: $random hasEatenKeys.length: ${hasEatenKeys.length}');

        var newHasEatenCount = hasEatenMap[key];
        var cravingHasEatenCount = hasEatenMap[currentlyCraving];
        
        if(key != eatenFoodId && WorldMap.calculateRandomFloat() < ServerSettings.YumFoodRestore)
        {
            hasEatenMap[key] -= ServerSettings.FoodReductionPerEating;
            newHasEatenCount = hasEatenMap[key];
            trace('IncreaseFoodValue: craving: hasEaten YES!!!: key: $key, ${newHasEatenCount}');

            if(newHasEatenCount <= 0 && cravings.contains(key) == false)
            {
                trace('IncreaseFoodValue: added craving: key: $key');
                cravings.push(key);
            }
        }
        else
        {
            trace('IncreaseFoodValue: craving hasEaten: NO!!!: key: $key, heldObject.id(): ${eatenFoodId}');
        }
            
        newHasEatenCount--;  // A food with full YUM is displayed as +1 craving 
        cravingHasEatenCount--; // A food with full YUM is displayed as +1 craving

        //if(newHasEatenCount >= 0) cravings.remove(eatenFoodId);
        //if(cravingHasEatenCount >= 0) cravings.remove(currentlyCraving);

        if(cravingHasEatenCount < 0 && currentlyCraving != 0 && currentlyCraving == eatenFoodId)
        {            
            trace('IncreaseFoodValue: craving: currentlyCraving: $currentlyCraving ${-cravingHasEatenCount}');

            this.connection.send(ClientTag.CRAVING, ['${currentlyCraving} ${-cravingHasEatenCount}']);
        }      
        else
        {
            /*else if(newHasEatenCount < 0)
            {
                this.connection.send(ClientTag.CRAVING, ['$key ${-newHasEatenCount}']);
                currentlyCraving = key;
            }*/
            

            if(cravings.length < 1 || WorldMap.calculateRandomFloat() < ServerSettings.YumNewCravingChance)
            {
                trace('IncreaseFoodValue: no new craving / choose random new: Eaten: ${eatenFoodId}');

                currentlyCraving = 0;

                // chose random new craving
                // TODO sort cravinglist by how difficult they are

                var index = 0;
                var foundNewCraving = false;

                for(i in 0...31)
                {
                    index = lastCravingIndex + WorldMap.calculateRandomInt(6 + i) - 3;

                    if(index == lastCravingIndex) index++;

                    if(index < 0) continue;
            
                    if(index >= ObjectData.foodObjects.length) continue;

                    var newObjData = ObjectData.foodObjects[index];

                    if(hasEatenMap[newObjData.id] > 0) continue;

                    foundNewCraving = true;

                    break;
                }

                if(foundNewCraving == false)
                {
                    trace('WARNING: No new random craving found!!!');
                    this.connection.send(ClientTag.CRAVING, ['${currentlyCraving} 0']); 
                    return;
                }

                var newObjData = ObjectData.foodObjects[index];

                if(hasEatenMap.exists(newObjData.id) == false) hasEatenMap[newObjData.id] = -1; // make sure to add it to the cravins and give a little boni

                newHasEatenCount = hasEatenMap[newObjData.id];
                newHasEatenCount--;

                trace('IncreaseFoodValue; new random craving: ${newObjData.description} ${newObjData.id} lastCravingIndex: $lastCravingIndex index: $index  newHasEatenCount: ${-newHasEatenCount}');

                lastCravingIndex = index;
                currentlyCraving = newObjData.id;

                this.connection.send(ClientTag.CRAVING, ['${currentlyCraving} ${-newHasEatenCount}']); 
            }
            else
            {
                // chose craving from known craving list
                var random = WorldMap.calculateRandomInt(cravings.length -1);
                var key = cravings[random];
                newHasEatenCount = hasEatenMap[key];
                newHasEatenCount--;
                this.connection.send(ClientTag.CRAVING, ['$key ${-newHasEatenCount}']);
                currentlyCraving = key;

                trace('IncreaseFoodValue: new craving: cravingHasEatenCount: $cravingHasEatenCount currentlyCraving: $currentlyCraving ${-newHasEatenCount}');
            }
        }            
    }


    private static function doSwitchCloths(playerFrom:GlobalPlayerInstance, playerTo:GlobalPlayerInstance, clothingSlot:Int) : Bool
    {
        var objClothingSlot = playerFrom.calculateClothingSlot();

        trace('self:o_id: ${playerFrom.o_id[0]} helobj.id: ${playerFrom.heldObject.id} clothingSlot: $clothingSlot objClothingSlot: $objClothingSlot');

        if(playerFrom.age < ServerSettings.MinAgeToEat && playerFrom.heldObject.id != 0)
        {
            trace('doSwitchCloths: playerFrom age ${playerTo.age} < ${ServerSettings.MinAgeToEat} cannot put on cloths');
            
            return false;
        }

        if(playerFrom != playerTo)
        {
            if(playerTo.age < ServerSettings.MaxAgeForAllowingClothAndPrickupFromOthers)
            {
                trace('doSwitchCloths: target player age ${playerTo.age} < ${ServerSettings.MaxAgeForAllowingClothAndPrickupFromOthers}');

                return false;
            }
        }

        if(objClothingSlot < 0 && playerFrom.heldObject.id != 0) return false;

        var array = playerTo.clothing_set.split(";");

        if(array.length < 6)
        {
            trace('WARNING! Clothing string missing slots: ${playerTo.clothing_set}' );

            return false;
        }  
   
        // if object is a shoe (objClothingSlot == 2) and if no clothingSlot is set, then use on empty foot if there is
        if(objClothingSlot == 2 && clothingSlot == -1)
        {
            if(playerTo.clothingObjects[2].id != 0 && playerTo.clothingObjects[3].id == 0) clothingSlot = 3;
            else clothingSlot = 2;
        }
        else
        {
            // if not a shoe use clothing slot from the held object if it has
            if(objClothingSlot > -1 && clothingSlot != 2 && clothingSlot != 3 ) clothingSlot = objClothingSlot;
        }

        trace('self: ${playerFrom.o_id[0]} clothingSlot: $clothingSlot objClothingSlot: $objClothingSlot');

        if(clothingSlot < 0) return false;        

        var tmpObj = playerTo.clothingObjects[clothingSlot];
        playerTo.clothingObjects[clothingSlot] = playerFrom.heldObject;
        playerFrom.setHeldObject(tmpObj);

        // switch clothing if there is a clothing on this slot
        //var tmp = Std.parseInt(array[clothingSlot]);
        array[clothingSlot] = '${playerTo.clothingObjects[clothingSlot].toString()}';
        playerTo.clothing_set = '${array[0]};${array[1]};${array[2]};${array[3]};${array[4]};${array[5]}';
        trace('this.clothing_set: ${playerTo.clothing_set}');

        playerFrom.action = 0;
 
        playerTo.SetTransitionData(playerTo.x, playerTo.y, true); // TODO needs to be set right
        
        Connection.SendUpdateToAllClosePlayers(playerTo);

        if(playerFrom != playerTo)
        {
            playerFrom.SetTransitionData(playerTo.x, playerTo.y, true); // TODO needs to be set right

            Connection.SendUpdateToAllClosePlayers(playerFrom);
        }

        //this.action = 0;

        return true;
    }

    private function calculateClothingSlot() : Int
    {
        var objClothingSlot = -1;

        if(this.o_id[0] != 0)
        {
            var objectData = ObjectData.getObjectData(this.o_id[0]);
            //trace("OD: " + objectData.toFileString());        

            switch objectData.clothing.charAt(0) {
                case "h": objClothingSlot = 0;      // head
                case "t": objClothingSlot = 1;      // torso
                case "s": objClothingSlot = 2;      // shoes
                //case "s": objClothingSlot = 3;    // shoes
                case "b": objClothingSlot = 4;      // skirt / trouser
                case "p": objClothingSlot = 5;      // backpack
            }

            trace('objectData.clothing: ${objectData.clothing}');
            trace('objClothingSlot:  ${objClothingSlot}');
            //trace('clothingSlot:  ${clothingSlot}');
        }

        return objClothingSlot;
    }

    public function doPlaceObjInClothing(clothingSlot:Int, isDrop:Bool = false) : Bool
    {
        if(clothingSlot < 0 ||  clothingSlot >= this.clothingObjects.length) return false;

        var clothing = this.clothingObjects[clothingSlot];

        if(TransitionHelper.DoContainerStuffOnObj(this, clothing, isDrop) == false) return false;

        setInClothingSet(clothingSlot);

        if(isDrop) return true; // currently flase if called from drop

        SetTransitionData(this.x, this.y, true);

        Connection.SendUpdateToAllClosePlayers(this);

        return true;
    }

    private function setInClothingSet(clothingSlot:Int)
    {
        var array = this.clothing_set.split(";");

        if(array.length < 6)
        {
            trace('Clothing string missing slots: ${this.clothing_set}' );
        }  

        array[clothingSlot] = '${clothingObjects[clothingSlot].toString()}';
        this.clothing_set = '${array[0]};${array[1]};${array[2]};${array[3]};${array[4]};${array[5]}';
    }

    /*
    SREMV x y c i#

    SREMV is special case of removing an object contained in a piece of worn 
      clothing.
      c specifies the clothing slot to remove from:  0=hat, 1=tunic, 
         2=frontShoe, 3=backShoe, 4=bottom, 5=backpack
      i specifies the index of the container item to remove, or -1 to
	  remove top of stack.
    */
    // SREMV -5 6 5 -1 remnove from backpack
    public function specialRemove(x:Int,y:Int,clothingSlot:Int,index:Null<Int>) : Bool
    {
        trace("SPECIAL REMOVE:");

        if(clothingSlot < 0)
        {
            this.connection.send(PLAYER_UPDATE,[this.toData()]);            
            return false;
        }
             
        var container = this.clothingObjects[clothingSlot];

        if(container.containedObjects.length < 1) 
        {
            this.connection.send(PLAYER_UPDATE,[this.toData()]);            
            return false;
        }

        this.mutex.acquire();

        if(ServerSettings.debug)
        {
            specialRemoveHelper(container, clothingSlot, index);
        }
        else{
            try
            {
                specialRemoveHelper(container, clothingSlot, index);
            }
            catch(ex)
            {
                trace('WARNING: $ex ' + ex.details);
            }
        }

        this.mutex.release();

        Connection.SendUpdateToAllClosePlayers(this);

        return true;
    }

    private function specialRemoveHelper(container:ObjectHelper, clothingSlot:Int,index:Null<Int>)
    {
        this.setHeldObject(container.removeContainedObject(index));

        setInClothingSet(clothingSlot);

        SetTransitionData(x,y, true);

        trace('this.clothing_set: ${this.clothing_set}');
    }

    public function isHoldingYum() : Bool
    {
        var heldObjData = heldObject.objectData;
        if(heldObjData.dummyParent != null) heldObjData = heldObjData.dummyParent;

        if(heldObjData.foodValue < 1) return false;

        var countEaten = hasEatenMap[heldObjData.id];

        return countEaten < ServerSettings.YumBonus; 
    }

    public function isHoldingMeh() : Bool
    {
        var heldObjData = heldObject.objectData;
        if(heldObjData.dummyParent != null) heldObjData = heldObjData.dummyParent;

        if(heldObjData.foodValue < 1) return false;

        var countEaten = hasEatenMap[heldObjData.id];

        return countEaten > ServerSettings.YumBonus; 
    }

    public function setHeldObject(obj:ObjectHelper)
    {
        this.heldObject = obj;

        MakeSureHoldObjIdAndDummyIsSetRightAndNullObjUsed();

        if(obj != null && obj.objectData.foodValue > 0)
        {
            if(this.isHoldingYum()) this.doEmote(Emote.joy);
            else this.doEmote(Emote.sad);
            //else if(isSuperMeh) playerTo.doEmote(Emote.ill);            
        }
    }

    public function MakeSureHoldObjIdAndDummyIsSetRightAndNullObjUsed()
    {
        if(this.o_id[0] < 0) return; // do nothing if a player is hold

        var obj = this.heldObject;

        if(obj == null)
        {
            obj = ObjectHelper.readObjectHelper(this, [0]);
            this.heldObject = obj;
        } 

        obj.TransformToDummy();
        this.o_id = obj.toArray();
        this.held_yum = isHoldingYum(); 
    }

    public function transformHeldObject(id:Int)
    {
        var toObjData = ObjectData.getObjectData(id);
        if(toObjData.dummyParent != null) toObjData = toObjData.dummyParent;

        var fromObjData = heldObject.objectData;
        if(fromObjData.dummyParent != null) fromObjData = fromObjData.dummyParent;

        
        if(toObjData.id != fromObjData.id)
        {
            heldObject.numberOfUses = 1;
            //TODO set to max numberOfUses??? heldObject.numberOfUses = heldObject.objectData

            trace('transformHeldObject: ${fromObjData.id} --> ${toObjData.id} / numberOfUses set to 1');
        }

        trace('transformHeldObject: heldObject.numberOfUses: ${heldObject.numberOfUses}');

        heldObject.id = id;
        setHeldObject(heldObject);
    }

    public function doDeath(deathReason:String)
    {
        if(this.deleted) return;

        Server.server.map.mutex.acquire();

        try
        {
            this.deleted = true;
            this.age = this.trueAge; // bad health and starving can influence health, so setback true time a player lifed so that he sees in death screen
            this.reason = deathReason;

            // TODO calculate score
            // TODO set coordinates player based
            ServerSettings.startingGx = this.tx();
            ServerSettings.startingGy = this.ty();

            //this.connection.die();
        
            placeGrave();

        }catch(ex)
        {
            trace('WARNING: ' + ex.details);
        }

        Server.server.map.mutex.release();
    }

    public function placeGrave()
    {
        var grave = new ObjectHelper(this, 87); // 87 = Fresh Grave 88 = grave

        if(this.heldObject != null)
        {
            grave.containedObjects.push(this.heldObject);
            this.setHeldObject(null);
        }

        // place the clothings in the grave, but not need to remove them from the player, since he is dead... //clothing_set:String = "0;0;0;0;0;0";
        for(obj in this.clothingObjects)
        {
            if(obj.id == 0) continue;

            grave.containedObjects.push(obj);
        }

        if(WorldMap.PlaceObject(this.tx(), this.ty(), grave) == false) trace('WARNING: could not place any grave for player: ${this.p_id}');
    }

    // insulation reaches from 0 to 2
    public function calculateClothingInsulation() : Float
    {
        var clothingInsulation:Float = 0;
        
        for(clothing in this.clothingObjects)
        {
            if(clothing.id == 0) continue;
             
            clothingInsulation += clothing.objectData.getInsulation();
            
            //trace('insulation: ${clothing.description} ${clothing.objectData.getInsulation()}');
        }

        //trace('clothingInsulation: $clothingInsulation');

        return clothingInsulation;
    }

    public function calculateClothingHeatProtection() : Float
    {
        var clothingHeatProtection:Float = 0;
        
        for(clothing in this.clothingObjects)
        {
            if(clothing.id == 0) continue;
                
            clothingHeatProtection += clothing.objectData.getHeatProtection();
            
            //trace('insulation: ${clothing.description} ${clothing.objectData.getInsulation()}');
        }

        //trace('clothingInsulation: $clothingInsulation');

        return clothingHeatProtection;
    }

    public function calculateFoodStoreMax() : Float
    {
        var p:GlobalPlayerInstance = this;
        var age = p.age;
        var food_store_max:Float = ServerSettings.GrownUpFoodStoreMax;

        if(age < 20) food_store_max = ServerSettings.NewBornFoodStoreMax + age / 20 * (ServerSettings.GrownUpFoodStoreMax - ServerSettings.NewBornFoodStoreMax);
        if(age > 50) food_store_max = ServerSettings.OldAgeFoodStoreMax + (60 - age) / 10 * (ServerSettings.GrownUpFoodStoreMax - ServerSettings.OldAgeFoodStoreMax);

        if(p.food_store < 0) food_store_max += ServerSettings.FoodStoreMaxReductionWhileStarvingToDeath * p.food_store;

        food_store_max -= p.hits;

        return food_store_max;
    }


    // BABY x y# // BABY x y id#
    /**BABY is special case of USE action taken on a baby to pick them up.
     They are dropped with the normal DROP action.
     NOTE the alternate call for BABY with extra id parameter.
     this specifies a specific person to pick up, if more than one is
     close to the target tile.**/
    public function doBaby(x:Int, y:Int, playerId:Int) : Bool // playerId = -1 if no specific player is slected
    {
        var done = false;
        var targetPlayer = getPlayerAt(this.tx() + x, this.tx() + y, playerId);

        trace('doBaby($x, $y playerId: $playerId)');

        if(isCloseToPlayer(targetPlayer) == false)
        {
            this.connection.send(PLAYER_UPDATE,[this.toData()]);

            trace('doBaby: x,y is too far away!');

            return false;
        }

        if(targetPlayer == null)
        {
            this.connection.send(PLAYER_UPDATE,[this.toData()]);

            trace('doBaby: could not find target player!');

            return false;
        }

        this.mutex.acquire();
        
        if(ServerSettings.debug)
        {
            done = doBabyHelper(x,y, targetPlayer);
        }
        else
        {
            try
            {
                done = doBabyHelper(x,y, targetPlayer);
            }
            catch(e)
            {                
                trace("WARNING: " + e);
            }
        }

        // send always PU so that player wont get stuck
        if(done == false)
        {
            this.connection.send(PLAYER_UPDATE,[this.toData()]);
            this.connection.send(FRAME);
        }

        this.mutex.release();

        return done;
    }

    public function doBabyHelper(x:Int, y:Int, player:GlobalPlayerInstance) : Bool
    {
        if(this.o_id[0] != 0)
        {
            trace('Cannot pickup player, since hands are not empty!');

            return false;
        }

        if(player.age >= ServerSettings.MaxAgeForAllowingClothAndPrickupFromOthers ) // TODO allow pickup of knocked out players 
        {
            trace('Cannot pickup player, player is too old! player.age: ${player.age}');

            return false;
        }

        this.heldPlayer = player;
        player.heldByPlayer = this;

        this.SetTransitionData(x,y,true);

        trace('doBabyHelper: o_id:  ${this.o_id}');

        Connection.SendUpdateToAllClosePlayers(this, true);

        return true;
    }

    public function dropPlayer() : Bool
    {
        trace('drop player');

        this.mutex.acquire();

        if(this.heldPlayer == null)
        {
            this.connection.send(PLAYER_UPDATE,[this.toData()]);
            this.mutex.release();
            return false;    
        }

        var done = doHelper(this, this.heldPlayer, dropPlayerHelper);

        this.mutex.release();

        return done;
    }

    private static function dropPlayerHelper(player:GlobalPlayerInstance) : Bool
    {
        trace('drop player helper');

        var heldPlayer = player.heldPlayer;

        heldPlayer.x = player.tx() - heldPlayer.gx;
        heldPlayer.y = player.ty() - heldPlayer.gy;

        player.heldPlayer = null;
        player.o_id = [0];

        heldPlayer.heldByPlayer = null;
        heldPlayer.forced = true;
        heldPlayer.responsible_id = player.p_id;
        heldPlayer.done_moving_seqNum += 1;

        Connection.SendUpdateToAllClosePlayers(player,true, false);
        Connection.SendUpdateToAllClosePlayers(heldPlayer);

        heldPlayer.forced = false;
        heldPlayer.responsible_id = -1;

        return true; 
    }

    /**
        JUMP is used by a baby to jump out of its mother's arms.  The x and y 
     coordinates are ignored.
     MOVE can NO LONGER be used to jump out of arms (it was the old way).
     It was less safe, because bad message interleavings server-side make
     MOVE ambiguous in the case of jump-out (for example, if the baby
     has already been dropped by the time the MOVE arrives, the server will
     interpret it as a legitimate move attempt).
     JUMP is also used to make an immobile baby wiggle on the ground.
    **/
    public function jump() : Bool
    {
        trace('jump');

        this.mutex.acquire();

        if(this.heldByPlayer == null)
        {
            this.connection.send(PLAYER_UPDATE,[this.toData()]);
            this.connection.sendWiggle(this);
            this.connection.send(FRAME, null, true);
            this.mutex.release();
            return false;    
        }

        var done = doHelper(this.heldByPlayer, this, dropPlayerHelper);

        this.mutex.release();

        return done;
    }

    private static function doHelper(player:GlobalPlayerInstance, targetPlayer:GlobalPlayerInstance, doFunction:GlobalPlayerInstance->Bool) : Bool
    {
        var done = false;
    
        // make sure that if both players at the same time try to interact with each other it does not end up in a dead lock 
        while(targetPlayer.mutex.tryAcquire() == false)
        {
            player.mutex.release();

            Sys.sleep(WorldMap.calculateRandomFloat() / 5);

            player.mutex.acquire();
        } 

        Macro.exception(done = doFunction(player));

        // send always PU so that player wont get stuck
        if(done == false)
        {
            player.connection.send(PLAYER_UPDATE,[player.toData()]);
            player.connection.send(FRAME);
        }

        targetPlayer.mutex.release();

        return done;
    }

    public function isFertile() : Bool
    {
        if(this.age < ServerSettings.MinAgeFertile || this.age > ServerSettings.MaxAgeFertile) return false;

        return isFemal();
    }

    public function isFemal()
    {
        var person = ObjectData.getObjectData(this.po_id);
        return person.male == false; 
    }

    public function isMale()
    {
        var person = ObjectData.getObjectData(this.po_id);
        return person.male; 
    }

    private static function DoDebugCommands(player:GlobalPlayerInstance, text:String)
    {
        if(text.indexOf('!HIT') != -1)
        {
            trace('!HIT');

            player.hits +=10;
            player.food_store_max = player.calculateFoodStoreMax();

            // reason_killed_id 
            if(player.food_store_max < 0)
            {
                player.doDeath('reason_killed_${player.woundedBy}');
            }
            else if(player.woundedBy == 0)
            {
                player.woundedBy = 418;
                player.connection.send(ClientTag.DYING, ['${player.p_id}']);
            }

            Connection.SendUpdateToAllClosePlayers(player);
        }
        else if(text.indexOf('!HEAL') != -1)
        {
            player.hits -=5;
            if(player.hits < 0) player.hits = 0; 

            player.food_store_max = player.calculateFoodStoreMax();

            if(player.woundedBy != 0 && player.hits < 1)
            {
                player.woundedBy = 0; 
                player.connection.send(ClientTag.HEALED, ['${player.p_id}']);
            }

            Connection.SendUpdateToAllClosePlayers(player);
        }
        /*else if(text.contains('!SWITCH')) // TODO needs support from client
        {
            var playerToSwitchTo = player.getClosestPlayer(20);

            if(playerToSwitchTo != null)
            {
                trace('switch platyer from: ${player.p_id} to ${playerToSwitchTo.p_id} ');

                player.connection.player = playerToSwitchTo;

                if(playerToSwitchTo.connection != null) playerToSwitchTo.connection.player = player;
                else{
                    playerToSwitchTo.serverAi.player = player;
                }
            }
        } */
        else if(text.indexOf('!CREATEALL') != -1)
        {
            Server.server.map.generateExtraDebugStuff(player.tx(), player.ty());
        }
        else if(text.indexOf('!CREATE') != -1) // "create xxx" with xxx = id
        {
            trace('Create debug object');

            var id = findObjectByCommand(text);

            if(id < 0) return;
            
            WorldMap.world.setObjectId(player.tx(), player.ty(), [id]);

            Connection.SendMapUpdateToAllClosePlayers(player.tx(), player.ty(), [id]);
        }
    }

    
    public static function findObjectByCommand(text:String) : Int
    {
        /* var startsWith = true;

        if(text.indexOf('!!') != -1)
        {
            startsWith = false;

            text = StringTools.replace(text, '!', '');
        }
        */

        var strings = text.split(' ');

        if(strings.length < 2) return -1;

        var id = Std.parseInt(strings[1]);

        //trace('${strings[1]} $id');

        var toSearch = StringTools.replace(text, '${strings[0]} ', '');        
        var end = toSearch.contains('!');
        toSearch = StringTools.replace(toSearch, '!', '');

        trace('Command Search: /${toSearch}/ end: $end');

        

        if(id == null)
        {
            for(obj in ObjectData.importedObjectData)
            {
                var description = obj.description.toUpperCase();
                description = StringTools.replace(description, '\n', '');
                description = StringTools.replace(description, '\r', '');

                if(description == toSearch)
                {
                    id = obj.id;
                    break;
                }
            }
        }

        if(id == null)
        {
            for(obj in ObjectData.importedObjectData)
            {
                var description = obj.description.toUpperCase();
                description = StringTools.replace(description, '\n', '');
                description = StringTools.replace(description, '\r', '');

                //trace('/${description}/');

                if(end)
                {
                    if(StringTools.endsWith(description, toSearch))
                    {
                        id = obj.id;
                        break;
                    }
                }
                else
                {
                    if(StringTools.startsWith(description, toSearch))
                    {
                        id = obj.id;
                        break;
                    }
                }
            }
        } 

        if(id == null)
        {
            for(obj in ObjectData.importedObjectData)
            {
                var description = obj.description.toUpperCase();
                description = StringTools.replace(description, '\n', '');
                description = StringTools.replace(description, '\r', '');

                if(description.indexOf(toSearch) != -1)
                {
                    id = obj.id;
                    break;
                }
            }
        } 

        if(id == null) return -1;

        return id;
    }

    public function isAi() : Bool
    {
        return this.serverAi != null;   
    }
}

// TODO give one at start
@:enum abstract Emote(Int) from Int to Int
{
    public var happy = 0;  // used YUM
    public var mad = 1; 
    public var angry = 2;
    public var sad = 3;  
    public var devious = 4;
    public var joy = 5;
    public var blush = 6; // redface
    public var yellowFever = 7; // TODO moskito
    public var snowSplat = 8;
    public var hubba = 9; // eyes
    public var ill = 10;  // TODO super meh food
    public var yoohoo = 11; //whistle
    public var hmph = 12; // used for eating MEH food
    public var love = 13; // TODO partner
    public var oreally = 14;
    public var shock = 15;
    public var murderFace = 16;
    public var tattooChest= 17;
    public var pneumonia = 18; // body white // used for cold
    public var biomeRelief = 19;
    public var dehydration = 20; // redpoints // TODO heat?
    public var heatStroke = 21; // used for super heat
    public var tattooBack = 22;
    public var tattooMouth = 23;
    public var tattooHead = 24;
    public var tattooFace = 25;
    public var tattooStomach = 26;
    public var terrified = 27;
    public var homesick = 28;
    public var spicyFood = 29; // TODO ?
    public var refuseFood = 30; // TODO ?
    public var starving = 31; // used for starving

    public var miamFood = 32; // used for eating craved food
    public var noHead = 33; // ?
    public var normal = 34; // ?
    public var moustache = 36; // ?
}


// TODO Arcurus>> add birth logic - suggestion:
    // select mother or Admam / Eve
    // if no mother 50% born as Adam 50 % born as Eve
    // First companion of Adam is Eve, of Eve it is Adam

    // TODO Arcurus>> "curses" function through dead bodies that are not properly burried
    // bone pile an normal grave blocks 200 Tiles nearby
    // bone pile dos not decay
    // grave with at least a grave stone block for 15 min
    // additional if you are blocked, you are shown "cursed" to others of you go near
    // for "cursed" your name is consantly shown in "cursed" color
    // "cursed" lowers your speed to 80% and pickup of Age 3 items (you can still use if you have one)
    // "cursed" hinders you to engage with your own dead body 
    // if you are blocked everywhere you may be born as "lowborn"

    // TODO Arcurus>> birth logic if you are not blocked
    // mothers on horses / cars cannot have children
    // mothers who where not close to a male in last 9 months cannot have a child 
    // mother must be at least 14 and max 40
    // X2 times chance for each grave with at least a gravestone nearby (100 Tiles)
    // X1/2 chance for each living child a mother has
    // X (score this life) / (average this live score of living players) (score is connected to YUM plus extra)
    
    // TODO Arcurus>> nobles and low born
    // If you are top 20% score of currently playing players (min 5 player) you are born as "noble"
    // If you are lowest 20% score of currently playing players (min 5 player) you are born as "low born"
    // as noble / low born first noble / low born mothers are considered
    // (new players have a 50% change of noble birth in their first 5 lifes)
    // nobels follow by default the leader
    // by default you follow your mother or / and??? father 50%
    // if your mother / father dies, you follow the noble of the mother / father
    // people in a village are distributed as followers among the nobles if a nobles dies
    
    // TODO Arcurus>> prince
    // if you have the highest score in this village (not counting the leader score) you are born as prince / princess to the leader
    // the eldest prince / princess becomes the crown prince
    // if there is no prince the noble with the highest score in this village becomes Cancelor
    // exiles / commands from crown prince / cancelor are valid for all followers if not overriden by the leader
    // giving a crown from the leader to a noble or prince makes them the new Cancelor / crown prince as long as he keeps the crown. 
    // A cancelor with a crown will get the new leader in case of the leaders death