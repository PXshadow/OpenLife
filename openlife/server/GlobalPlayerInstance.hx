package openlife.server;
import sys.io.File;
import openlife.auto.Ai;
import haxe.Exception;
import openlife.server.Lineage.PrestigeClass;
import openlife.auto.AiHelper;
import openlife.server.Biome.BiomeTag;
import openlife.macros.Macro;
import openlife.data.transition.TransitionImporter;
import openlife.auto.WorldInterface;
import openlife.auto.PlayerInterface;
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

// TODO give one at start?
@:enum abstract Emote(Int) from Int to Int
{
    public var happy = 0;  // eating YUM
    public var mad = 1; 
    public var angry = 2;
    public var sad = 3;  // eating meh // holding SUPERMEH
    public var devious = 4;
    public var joy = 5; // holding YUM
    public var blush = 6; // redface
    public var yellowFever = 7; // TODO moskito
    public var snowSplat = 8;
    public var hubba = 9; // eyes
    public var ill = 10;  // eating SUPERMEH food
    public var yoohoo = 11; //whistle
    public var hmph = 12; // holding MEH
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
    public var refuseFood = 30; // used if food is reused
    public var starving = 31; // used for starving

    public var miamFood = 32; // used for eating craved food
    public var noHead = 33; // ?
    public var normal = 34; // ?
    public var moustache = 36; // ?
    //4313 Satisfied Mouth???
    //4184 Starving Mouth
}

// GlobalPlayerInstance is used as a WorldInterface for an AI, since it may be limited what the AI can see so player information is relevant
@:rtti
class GlobalPlayerInstance extends PlayerInstance implements PlayerInterface implements WorldInterface
{
    public static var AllPlayerMutex = new Mutex();

    public static var AllPlayers = new Map<Int,GlobalPlayerInstance>();
    public static function AddPlayer(player:GlobalPlayerInstance)
    {
        AllPlayers[player.p_id] = player;
        Lineage.AddLineage(player.p_id, player.lineage);
    } 

    public static var medianPrestige:Float = ServerSettings.HealthFactor;

    public static var lastAiEveOrAdam:GlobalPlayerInstance; 
    public static var lastHumanEveOrAdam:GlobalPlayerInstance; 
    public static var LastLeaderBadgeColor:Int = 0;

    public var lineage:Lineage;

    private var myMother:GlobalPlayerInstance;
    private var myFather:GlobalPlayerInstance;

    // make sure to set these null is player is deleted so that garbage collector can clean up
    public var followPlayer:GlobalPlayerInstance;
    public var heldPlayer:GlobalPlayerInstance;
    public var heldByPlayer:GlobalPlayerInstance;

    public function getFollowPlayer(){return followPlayer;}
    public function getHeldPlayer(){return heldPlayer;}
    public function getHeldByPlayer(){return heldByPlayer;}

    // handles all the movement stuff
    public var moveHelper:MoveHelper;
    public var killMode:Bool = false;

    // additional ObjectInformation for the object stored in backpack or other clothing. The size is 6 since 6 clothing slots
    public var clothingObjects:Vector<ObjectHelper> = new Vector(6);  

    public var connection:Connection; 

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

    // exhaustion
    public var exhaustion:Float = 0; 

    // birth stuff 
    public var childrenBirthMali:Float = 0;  // increases for each child // reduces for dead childs

    public var foodUsePerSecond = ServerSettings.FoodUsePerSecond; // is changed in update temperature

    public var exiledByPlayers = new Map<Int, GlobalPlayerInstance>();

    public var coins:Float = 0;

    public var prestigeFromChildren:Float = 0;
    public var prestigeFromEating:Float = 0;
    public var prestigeFromFollowers:Float = 0;
    public var prestigeFromWealth:Float = 0;
    public var prestigeFromParents:Float = 0; // TODO not saved yet
    public var prestigeFromSiblings:Float = 0; // TODO not saved yet

    // list of objects the player owns like gates
    public var owning:Array<ObjectHelper> = new Array<ObjectHelper>(); 

    // combat stuff
    public var lastPlayerAttackedMe:GlobalPlayerInstance = null; 
    public var lastAttackedPlayer:GlobalPlayerInstance = null; // used to exile ally if attacked twice
    public var angryTime:Float = ServerSettings.CombatAngryTimeBeforeAttack; // before one attacks without he or an ally beeing attacked first he must be angry a certain time

    public var newFollower:GlobalPlayerInstance = null;
    public var newFollowerFor:GlobalPlayerInstance = null;
    public var newFollowerTime:Float = 0; 

    public var isCursed:Bool = false;
    public var inWrongBiome = false;
    public var inHomeBiome = false;

    public var jumpedTiles:Float = 0; // used to limit how often a player can "jump" per second.

    public var hiddenWound:ObjectHelper = null;
    public var fever:ObjectHelper = null; // used for yellow fever
    public var yellowfeverCount:Float = 0;

    public var lastSayInSec:Float = 0;

    // set all stuff null so that nothing is hanging around
    public function delete()
    {
        this.deleted = true;

        // need grandmother to inherit eaten food counts to grantkids
        if(this.myMother != null) this.myMother.myMother = null;
        if(this.myFather != null) this.myFather.myFather = null;

        this.followPlayer = null;
        this.heldPlayer = null;
        this.heldByPlayer = null;
    
        this.exiledByPlayers =  new Map<Int, GlobalPlayerInstance>();

        this.lastAttackedPlayer = null;
        this.lastPlayerAttackedMe = null;

        AllPlayers.remove(this.p_id);
    }

    private static function GetName(objId:Int) : String
    {
        return ObjectData.getObjectData(objId).name;
    }

    public static function WriteAllPlayers(path:String)
    {
        WritePlayers(path, AllPlayers);
    }

    public static function WritePlayers(path:String, players:Map<Int,GlobalPlayerInstance>)
    { 
        var count = 0;
        var dataVersion = 1;
        var writer = File.write(path, true);

        for(player in players) count++;

        writer.writeInt32(dataVersion); 
        writer.writeInt32(count);
        
        for(obj in players)
        {
            writer.writeInt32(obj.account.id); //account 166

            // write player instance variables
            writer.writeInt32(obj.p_id);
            writer.writeFloat(obj.food_store);
            writer.writeFloat(obj.food_store_max);
            writer.writeInt32(obj.last_ate_fill_max);
            writer.writeFloat(obj.yum_bonus);
            writer.writeFloat(obj.yum_multiplier);
            writer.writeInt32(obj.gx);
            writer.writeInt32(obj.gy);
            //writer.writeInt32(obj.tx);
            //writer.writeInt32(obj.ty);
            writer.writeInt32(obj.po_id);
            writer.writeInt32(obj.facing);
            writer.writeInt32(obj.action);
            writer.writeInt32(obj.action_target_x);
            writer.writeInt32(obj.action_target_y);
            writer.writeInt16(obj.o_id.length);
            for(i in obj.o_id) writer.writeInt32(i);
            writer.writeInt32(obj.o_origin_valid);
            writer.writeInt32(obj.o_origin_x);
            writer.writeInt32(obj.o_origin_y);
            writer.writeInt32(obj.o_transition_source_id);
            writer.writeFloat(obj.heat);
            writer.writeInt32(obj.done_moving_seqNum);
            writer.writeInt8(obj.forced ? 1 : 0);
            writer.writeInt32(obj.x);
            writer.writeInt32(obj.y);
            writer.writeFloat(obj.age);
            writer.writeFloat(obj.age_r);
            writer.writeFloat(obj.move_speed);
            writer.writeInt16(obj.clothing_set.length);
            writer.writeString(obj.clothing_set);
            writer.writeInt32(obj.just_ate);
            writer.writeInt32(obj.last_ate_id);
            writer.writeInt32(obj.responsible_id);
            writer.writeInt8(obj.held_yum ? 1 : 0);
            writer.writeInt8(obj.held_learned ? 1 : 0);
            writer.writeInt8(obj.deleted ? 1 : 0);
            writer.writeInt16(obj.reason.length);
            writer.writeString(obj.reason);
            writer.writeInt32(obj.i);
            //writer.writeInt16(obj.name.length);
            //writer.writeString(obj.name);
            //TODO? var a:Array<String>;
            ObjectHelper.WriteToFile(obj.heldObject, writer);

            // From GlobalPlayerInstance
            //lineage 1 CClass(openlife.server.Lineage,[])
            writer.writeInt32(GetPlayerIdForWrite(obj.myMother)); //2
            writer.writeInt32(GetPlayerIdForWrite(obj.myFather)); //3
            //myChildren 4 CClass(Array,[CClass(openlife.server.GlobalPlayerInstance,[])])
            writer.writeInt32(GetPlayerIdForWrite(obj.followPlayer)); //5
            writer.writeInt32(GetPlayerIdForWrite(obj.heldPlayer)); //6
            writer.writeInt32(GetPlayerIdForWrite(obj.heldByPlayer)); //7
            //moveHelper 11 CClass(openlife.server.MoveHelper,[])
            writer.writeInt8(obj.killMode ? 1 : 0);
            //clothingObjects 13 CAbstract(haxe.ds.Vector,[CClass(openlife.data.object.ObjectHelper,[])])
            //mutex 14 CAbstract(sys.thread.Mutex,[])
            //connection 15 CClass(openlife.server.Connection,[])
            writer.writeFloat(obj.trueAge);
            //hasEatenMap 17 CAbstract(haxe.ds.Map,[CAbstract(Int,[]),CAbstract(Float,[])])
            writer.writeInt32(obj.leaderBadgeColor);
            writer.writeInt32(obj.currentlyCraving);
            writer.writeInt32(obj.lastCravingIndex);
            writer.writeInt16(obj.cravings.length);
            for(i in obj.cravings) writer.writeInt32(i);
            writer.writeFloat(obj.hits);
            writer.writeInt32(obj.woundedBy);
            writer.writeFloat(obj.exhaustion);
            writer.writeFloat(obj.childrenBirthMali);
            writer.writeFloat(obj.foodUsePerSecond);
            //exiledByPlayers 27 CAbstract(haxe.ds.Map,[CAbstract(Int,[]),CClass(openlife.server.GlobalPlayerInstance,[])])
            writer.writeFloat(obj.coins);
            writer.writeFloat(obj.prestigeFromChildren);
            writer.writeFloat(obj.prestigeFromEating);
            writer.writeFloat(obj.prestigeFromFollowers);
            writer.writeFloat(obj.prestigeFromWealth);

            //writer.writeInt16(-1000); // write end sign

            //owning 33 CClass(Array,[CClass(openlife.data.object.ObjectHelper,[])])
            writer.writeInt32(GetPlayerIdForWrite(obj.lastPlayerAttackedMe)); //34
            writer.writeInt32(GetPlayerIdForWrite(obj.lastAttackedPlayer)); //35
            writer.writeFloat(obj.angryTime);
            writer.writeInt32(GetPlayerIdForWrite(obj.newFollower)); //37
            writer.writeInt32(GetPlayerIdForWrite(obj.newFollowerFor)); //38
            writer.writeFloat(obj.newFollowerTime);
            writer.writeInt8(obj.isCursed ? 1 : 0);
            writer.writeInt8(obj.inWrongBiome ? 1 : 0);
            writer.writeInt8(obj.inHomeBiome ? 1 : 0);
            writer.writeFloat(obj.jumpedTiles);
            ObjectHelper.WriteToFile(obj.hiddenWound, writer); //44
            ObjectHelper.WriteToFile(obj.fever, writer); //45
            writer.writeFloat(obj.yellowfeverCount);
            writer.writeFloat(obj.lastSayInSec);
            //writer.writeInt32(obj.id);
            //writer.writeInt16(obj.familyName.length);
            //writer.writeString(obj.familyName);
            //writer.writeInt32(GetPlayerIdForWrite(obj.mother)); //55
            //writer.writeInt32(GetPlayerIdForWrite(obj.father)); //58
            //writer.writeFloat(obj.lineagePrestige);
            //writer.writeFloat(obj.prestige);
            //account 166 CClass(openlife.server.PlayerAccount,[])

            //writer.writeInt16(-1000); // write end sign

            //clothingObjects 13
            writer.writeInt16(obj.clothingObjects.length);
            for(o in obj.clothingObjects)
            {
                ObjectHelper.WriteToFile(o, writer);
            }
            //writer.writeInt16(-1000); // write end sign

            //hasEatenMap 17            
            var keys = obj.hasEatenMap.keys();
            var length = 0;
            for(key in keys) length++;
            writer.writeInt16(length);
            //trace('write has eaten length: ${length}');
            var keys = obj.hasEatenMap.keys();
            for(key in keys)
            {
                writer.writeInt32(key);
                writer.writeFloat(obj.hasEatenMap[key]);

                //trace('write has eaten: ${key}');
            }

            //writer.writeInt16(-1000); // write end sign
            // exiledByPlayers 27
            var keys = obj.exiledByPlayers.keys();
            var length = 0;
            for(key in keys) length++;
            writer.writeInt16(length);
            for(p in obj.exiledByPlayers)
            {
                writer.writeInt32(p.p_id);
            }

            writer.writeInt16(-1000); // write end sign (just for checking if data is valid)

            // owning 33 is filled in InitObjectHelpersAfterRead 
            
            //DONE lineage 1 CClass(openlife.server.Lineage,[])
            //DONE moveHelper 11 CClass(openlife.server.MoveHelper,[])
            //DONE clothingObjects 13 CAbstract(haxe.ds.Vector,[CClass(openlife.data.object.ObjectHelper,[])])
            //DONE mutex 14 CAbstract(sys.thread.Mutex,[])
            //DONE connection 15 CClass(openlife.server.Connection,[])
            //DONE hasEatenMap 17 CAbstract(haxe.ds.Map,[CAbstract(Int,[]),CAbstract(Float,[])])
            //DONE exiledByPlayers 27 CAbstract(haxe.ds.Map,[CAbstract(Int,[]),CClass(openlife.server.GlobalPlayerInstance,[])])
            //DONE owning 33 CClass(Array,[CClass(openlife.data.object.ObjectHelper,[])])
            //DONE account 166 CClass(openlife.server.PlayerAccount,[])            
        }

        writer.close();

        //trace('wrote $count Players...');
        if(ServerSettings.DebugWrite) trace('wrote $count Players...');
    }

    private static function GetPlayerIdForWrite(player:GlobalPlayerInstance) : Int
    {
        return player == null ? -100 : player.p_id;
    }

    private static function GetPlayerFromId(playerId:Int) : GlobalPlayerInstance
    {
        if(playerId == -100) return null;
        return AllPlayers[playerId];
    }
    
    public static function ReadPlayers(path:String) : Map<Int,GlobalPlayerInstance>
    {
        var reader = File.read(path, true);
        var expectedDataVersion = 1;
        var dataVersion = reader.readInt32();
        var count = reader.readInt32();
        var loadedPlayers = new Map<Int,GlobalPlayerInstance>();
        var playersToLoad = new Map<Int, Map<String, Int>>();
        var exiledPlayersToLoad = new Map<Int, Array<Int>>();

        //return null;

        trace('Read players from file: $path count: ${count}');

        if(dataVersion != expectedDataVersion) throw new Exception('ReadPlayers: Data version is: $dataVersion expected data version is: $expectedDataVersion');

        //try{
            for(i in 0...count)
            {
                var accountId = reader.readInt32();
                var id = reader.readInt32();
                var connection = new Connection(null, Server.server);
                connection.playerAccount = PlayerAccount.GetPlayerAccountById(accountId);
                
                var obj = new GlobalPlayerInstance(connection, id);
                connection.serverAi = new ServerAi(obj); // use AI untill player logs in

                obj.lineage = Lineage.GetLineage(id);
                loadedPlayers[id] = obj;
                playersToLoad[id] = new Map<String, Int>();

                // read Playerinstance variables
                obj.food_store = reader.readFloat();
                obj.food_store_max = reader.readFloat();
                obj.last_ate_fill_max = reader.readInt32();
                obj.yum_bonus = reader.readFloat();
                obj.yum_multiplier = reader.readFloat();
                obj.gx = reader.readInt32();
                obj.gy = reader.readInt32();
                //obj.tx = reader.readInt32();
                //obj.ty = reader.readInt32();                
                obj.po_id = reader.readInt32();
                obj.facing = reader.readInt32();
                obj.action = reader.readInt32();
                obj.action_target_x = reader.readInt32();
                obj.action_target_y = reader.readInt32();
                obj.o_id = new Array<Int>();
                var len = reader.readInt16();
                for(i in 0...len){obj.o_id[i] = reader.readInt32();}
                obj.o_origin_valid = reader.readInt32();
                obj.o_origin_x = reader.readInt32();
                obj.o_origin_y = reader.readInt32();
                obj.o_transition_source_id = reader.readInt32();
                obj.heat = reader.readFloat();
                obj.done_moving_seqNum = reader.readInt32();
                obj.forced = reader.readInt8() != 0 ? true : false;
                obj.x = reader.readInt32();
                obj.y = reader.readInt32();
                obj.age = reader.readFloat();
                obj.age_r = reader.readFloat();
                obj.move_speed = reader.readFloat();
                var len = reader.readInt16();
                obj.clothing_set = reader.readString(len);
                obj.just_ate = reader.readInt32();
                obj.last_ate_id = reader.readInt32();
                obj.responsible_id = reader.readInt32();
                obj.held_yum = reader.readInt8() != 0 ? true : false;
                obj.held_learned = reader.readInt8() != 0 ? true : false;
                obj.deleted = reader.readInt8() != 0 ? true : false;
                var len = reader.readInt16();
                obj.reason = reader.readString(len);
                obj.i = reader.readInt32();
                //var len = reader.readInt16();
                //obj.name = reader.readString(len);
                //TODO? var a:Array<String>;
                obj.heldObject = ObjectHelper.ReadFromFile(reader);

                // read GlobalPlayerInstance
                //lineage 1 CClass(openlife.server.Lineage,[])
                playersToLoad[obj.p_id]["myMother"] = reader.readInt32(); //2
                playersToLoad[obj.p_id]["myFather"] = reader.readInt32(); //3
                //myChildren 4 CClass(Array,[CClass(openlife.server.GlobalPlayerInstance,[])])
                playersToLoad[obj.p_id]["followPlayer"] = reader.readInt32(); //5
                playersToLoad[obj.p_id]["heldPlayer"] = reader.readInt32(); //6
                playersToLoad[obj.p_id]["heldByPlayer"] = reader.readInt32(); //7
                //moveHelper 11 CClass(openlife.server.MoveHelper,[])
                obj.killMode = reader.readInt8() != 0 ? true : false;
                //clothingObjects 13 CAbstract(haxe.ds.Vector,[CClass(openlife.data.object.ObjectHelper,[])])
                //mutex 14 CAbstract(sys.thread.Mutex,[])
                //connection 15 CClass(openlife.server.Connection,[])
                obj.trueAge = reader.readFloat();
                //hasEatenMap 17 CAbstract(haxe.ds.Map,[CAbstract(Int,[]),CAbstract(Float,[])])
                obj.leaderBadgeColor = reader.readInt32();
                obj.currentlyCraving = reader.readInt32();
                obj.lastCravingIndex = reader.readInt32();
                obj.cravings = new Array<Int>();
                var len = reader.readInt16();
                for(i in 0...len){obj.cravings[i] = reader.readInt32();}
                obj.hits = reader.readFloat();
                obj.woundedBy = reader.readInt32();
                obj.exhaustion = reader.readFloat();
                obj.childrenBirthMali = reader.readFloat();
                obj.foodUsePerSecond = reader.readFloat();
                //exiledByPlayers 27 CAbstract(haxe.ds.Map,[CAbstract(Int,[]),CClass(openlife.server.GlobalPlayerInstance,[])])
                obj.coins = reader.readFloat();
                obj.prestigeFromChildren = reader.readFloat();
                obj.prestigeFromEating = reader.readFloat();
                obj.prestigeFromFollowers = reader.readFloat();
                obj.prestigeFromWealth = reader.readFloat();

                /*var end = reader.readInt16(); // read end sign
                if(end != -1000)
                {   
                    var message = 'read ${i+1} Players wrong end sign 1-1: ${end}';
                    trace(message);
                    throw new Exception(message);
                }*/

                //owning 33 CClass(Array,[CClass(openlife.data.object.ObjectHelper,[])])
                playersToLoad[obj.p_id]["lastPlayerAttackedMe"] = reader.readInt32(); //34
                playersToLoad[obj.p_id]["lastAttackedPlayer"] = reader.readInt32(); //35
                obj.angryTime = reader.readFloat();
                playersToLoad[obj.p_id]["newFollower"] = reader.readInt32(); //37
                playersToLoad[obj.p_id]["newFollowerFor"] = reader.readInt32(); //38
                obj.newFollowerTime = reader.readFloat();
                obj.isCursed = reader.readInt8() != 0 ? true : false;
                obj.inWrongBiome = reader.readInt8() != 0 ? true : false;
                obj.inHomeBiome = reader.readInt8() != 0 ? true : false;
                obj.jumpedTiles = reader.readFloat();
                obj.hiddenWound = ObjectHelper.ReadFromFile(reader); //44
                obj.fever = ObjectHelper.ReadFromFile(reader); //45
                obj.yellowfeverCount = reader.readFloat();
                obj.lastSayInSec = reader.readFloat();
                //obj.id = reader.readInt32();
                //var len = reader.readInt16();
                //obj.familyName = reader.readString(len);
                //playersToLoad[obj.p_id]["mother"] = reader.readInt32(); //55
                //playersToLoad[obj.p_id]["father"] = reader.readInt32(); //58
                //obj.lineagePrestige = reader.readFloat();
                //obj.prestige = reader.readFloat();
                //account 166 CClass(openlife.server.PlayerAccount,[])    
                
                obj.moveHelper.exactTx = obj.tx;
                obj.moveHelper.exactTy = obj.ty;

                /*var end = reader.readInt16(); // read end sign
                if(end != -1000)
                {   
                    var message = 'read ${i+1} Players wrong end sign2: ${end}';
                    trace(message);
                    throw new Exception(message);
                }*/

                //clothingObjects 13
                var len = reader.readInt16();
                for(i in 0...len)
                {
                    obj.clothingObjects[i] = ObjectHelper.ReadFromFile(reader);
                }

                /*var end = reader.readInt16(); // read end sign
                if(end != -1000)
                {   
                    var message = 'read ${i+1} Players wrong end sign: ${end}';
                    trace(message);
                    throw new Exception(message);
                }*/

                //hasEatenMap 17 
                var len = reader.readInt16();
                //trace('read has eaten length: ${len}');
                for(i in 0...len)
                {                    
                    var key = reader.readInt32();
                    obj.hasEatenMap[key] = reader.readFloat();
                    //trace('read has eaten: ${key}');
                }

                /*var end = reader.readInt16(); // read end sign
                if(end != -1000)
                {   
                    var message = 'read ${i+1} Players wrong end sign: ${end}';
                    trace(message);
                    throw new Exception(message);
                }*/

                // exiledByPlayers 27
                exiledPlayersToLoad[obj.p_id] = new Array<Int>();
                var len = reader.readInt16();
                for(i in 0...len)
                {
                    var key = reader.readInt32();
                    exiledPlayersToLoad[obj.p_id].push(key);
                }

                var end = reader.readInt16(); // read end sign
                if(end != -1000)
                {   
                    var message = 'read ${i+1} Players wrong end sign: ${end}';
                    trace(message);
                    throw new Exception(message);
                }

                if(obj.lineage == null)
                {
                    trace('read ${i+1} No Lineage found: ${obj.p_id}');
                }
                else trace('read ${i+1} Players... ${obj.name} account: ${obj.account.id} ${obj.account.email}');
            }
        /*}
        catch(ex)
        {
            reader.close();
            throw ex;
        }*/

        reader.close();

        AllPlayers = loadedPlayers;

        for(obj in loadedPlayers)
        {
            obj.myMother = GetPlayerFromId(playersToLoad[obj.p_id]["myMother"]); //2
            obj.myFather = GetPlayerFromId(playersToLoad[obj.p_id]["myFather"]); //3
            obj.followPlayer = GetPlayerFromId(playersToLoad[obj.p_id]["followPlayer"]); //5
            obj.heldPlayer = GetPlayerFromId(playersToLoad[obj.p_id]["heldPlayer"]); //6
            obj.heldByPlayer = GetPlayerFromId(playersToLoad[obj.p_id]["heldByPlayer"]); //7
            obj.lastPlayerAttackedMe = GetPlayerFromId(playersToLoad[obj.p_id]["lastPlayerAttackedMe"]); //34
            obj.lastAttackedPlayer = GetPlayerFromId(playersToLoad[obj.p_id]["lastAttackedPlayer"]); //35
            obj.newFollower = GetPlayerFromId(playersToLoad[obj.p_id]["newFollower"]); //37
            obj.newFollowerFor = GetPlayerFromId(playersToLoad[obj.p_id]["newFollowerFor"]); //38
            //obj.mother = GetPlayerFromId(playersToLoad[obj.p_id]["mother"]); //55
            //obj.father = GetPlayerFromId(playersToLoad[obj.p_id]["father"]); //58 
            
            var exiled = exiledPlayersToLoad[obj.p_id];
            for(id in exiled)
            {
                obj.exiledByPlayers[id] = AllPlayers[id];
                trace('read exiled Players ${obj.name} --> Exiled by: ${obj.exiledByPlayers[id].name}');
            }
        }

        trace('read $count Players...');

        return loadedPlayers;
    }

    public var id(get, null):Int;
    public function get_id(){return p_id;}

    //public override var name(get, set):String;
    public override function get_name(){return lineage.name;}
    public override function set_name(newName:String){return lineage.name = newName;}

    public var familyName(get, null):String;

    public function get_familyName()
    {
        return lineage.familyName;
    }

    public var mother(get, set):GlobalPlayerInstance;

    public function get_mother()
    {   
        return myMother;
    }

    public function set_mother(newMother:GlobalPlayerInstance)
    {
        this.myMother = newMother;
        lineage.mother = newMother;
        return this.myMother;
    }

    public var father(get, set):GlobalPlayerInstance;

    public function get_father()
    {
        return myFather;
    }

    public function set_father(newFather:GlobalPlayerInstance)
    {
        this.myFather = newFather;
        lineage.father = newFather;
        return this.myFather;
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

    public static function CreateNewHumanPlayer(c:Connection) : GlobalPlayerInstance
    {
        return new GlobalPlayerInstance(c);
    }

    public static function CreateNewAiPlayer(c:Connection) : GlobalPlayerInstance
    {
        var player = null;

        GlobalPlayerInstance.AllPlayerMutex.acquire();

        Macro.exception(player = new GlobalPlayerInstance(c));

        GlobalPlayerInstance.AllPlayerMutex.release();

        return player;
    }

    public function setObjectId(new_po_id:Int)
    {
        this.po_id = new_po_id;
        this.lineage.po_id = new_po_id;
    }

    private function new(c:Connection, loadedId:Int = -1)
    {
        super([]);

        if(c != null) c.player = this;

        this.connection = c;
        this.moveHelper = new MoveHelper(this);

        // if loaded from file set only id;
        if(loadedId > 0)
        {
            this.p_id = loadedId;
            return;
        }

        this.p_id = Server.server.playerIndex++;
        this.po_id = ObjectData.personObjectData[WorldMap.calculateRandomInt(ObjectData.personObjectData.length-1)].id;
        this.heldObject = ObjectHelper.readObjectHelper(this, [0]);        
        this.age_r = ServerSettings.AgingSecondsPerYear;
        this.lineage = new Lineage(this);

        this.lineage.prestigeClass = calculatePrestigeClass();
        
        for(i in 0...this.clothingObjects.length)
        {
            this.clothingObjects[i] = ObjectHelper.readObjectHelper(this, [0]);
        }    
        
        if(lastAiEveOrAdam != null && lastAiEveOrAdam.deleted) lastAiEveOrAdam = null; 
        if(lastHumanEveOrAdam != null && lastHumanEveOrAdam.deleted) lastHumanEveOrAdam = null;

        // TODO on big map dont spawn eve too far away
        // spawn human eve to human adam and ai eve to ai adam except if player count is very few 
        var isAi = this.isAi();
        var allowHumanSpawnToAIandAiToHuman = GetNumberLifingPlayers() <= ServerSettings.MaxPlayersBeforeStartingAsChild;
        var spawnEve =  (isAi && lastAiEveOrAdam != null) || (isAi == false && lastHumanEveOrAdam != null);
        spawnEve = isAi == false && ServerSettings.EveOrAdamBirthChance <= WorldMap.calculateRandomFloat() ? true : spawnEve;

        //if(false) spawnAsEve(allowHumanSpawnToAIandAiToHuman);
        if(spawnEve) spawnAsEve(allowHumanSpawnToAIandAiToHuman);
        else
        {
            if(spawnAsChild() == false) spawnAsEve(allowHumanSpawnToAIandAiToHuman);
        }        

        move_speed = ServerSettings.InitialPlayerMoveSpeed; // MoveHelper.calculateSpeed(this, gx, gy); // DOES not set grave curse if done at init
        
        food_store_max = calculateFoodStoreMax();
        food_store = food_store_max / 2;
        yum_multiplier = this.account.totalScore * ServerSettings.BirthPrestigeFactor;
        yum_multiplier = Math.max(yum_multiplier, (medianPrestige / 30) * trueAge);

        AddPlayer(this); // add after player is inited and before linage data like familyName from this player is used

        Connection.SendUpdateToAllClosePlayers(this); 

        for(c in Connection.getConnections())
        {
            c.send(ClientTag.NAME,['${this.p_id} ${this.name} ${this.familyName}']);
        }

        for(c in Connection.getConnections())
        {
            c.send(ClientTag.LINEAGE,[this.lineage.createLineageString()]);
        }

        Connection.SendFollowingToAll(this);

        if(this.mother != null && this.age < ServerSettings.MinAgeToEat)
        {          
            if(this.mother.isAi() == false) mother.connection.sendMapLocation(this,'BABY', 'baby');
            else mother.connection.serverAi.newChild(this);
        }    
    }

    // TODO test
    private function calculatePrestigeClass() : PrestigeClass
    {   
        //trace('PRESTIGE ${playerAccount.totalScore} prestigeNeededForNobleBirth: $prestigeNeededForNobleBirth');
        // [for(key in map.keys()) key]
        var players = [for(p in AllPlayers) p];

        if(players.length < 2) return PrestigeClass.Commoner;

        players.sort(function(a, b) {
            if(a.lineagePrestige < b.lineagePrestige) return -1;
            else if(a.lineagePrestige > b.lineagePrestige) return 1;
            else return 0;
        });

        for(p in players) trace('${p.name} PRESTIGE: ${p.lineagePrestige}');        

        var neededPrestige = CalculateNeededPrestige(players, 0.4);
        medianPrestige = Math.max(neededPrestige, ServerSettings.HealthFactor); // is needed for calculating health
        if(this.account.totalScore < neededPrestige) return PrestigeClass.Serf;

        if(players.length < 5) return PrestigeClass.Commoner;

        var neededPrestige = CalculateNeededPrestige(players, 0.8);
        if(this.account.totalScore < neededPrestige) return PrestigeClass.Commoner;

        return PrestigeClass.Noble;
    }

    public static function CalculateNeededPrestige(players:Array<GlobalPlayerInstance>, percent:Float = 0.4) : Float
    {
        var count = 0;

        for(p in players)
        {
            count++;

            if(count < players.length * percent) continue;

           trace('NEEDED PRESTIGE ${p.lineagePrestige} percent: ${percent}');

            return p.lineagePrestige;
        }

        return 999999;
    }

    private function spawnAsEve(allowHumanSpawnToAIandAiToHuman:Bool)
    {    
        var isAi = this.isAi();
        var lastEveOrAdam = isAi ? lastAiEveOrAdam : lastHumanEveOrAdam;

        trace('Spawn As Eve: ${this.account.email}');

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
        //var banaPlants = [for(key in hasEatenMap.keys()) key];
        var berryBushesTmp = [for(obj in WorldMap.world.berryBushes) obj];
        var bananaPlantsTmp = [for(obj in WorldMap.world.bananaPlants) obj];
        var wildCarrotsTmp = [for(obj in WorldMap.world.wildCarrots) obj];
        var cactusesTmp = [for(obj in WorldMap.world.cactuses) obj];
        var wildGarlicsTmp = [for(obj in WorldMap.world.wildGarlics) obj];

        //trace('spawnAsEve: banaPlants: ${bananaPlantsTmp.length} berryBushes: ${berryBushesTmp.length}');
        
        if(ServerSettings.SpwanAtLastDead || bananaPlantsTmp.length + berryBushesTmp.length < 10)
        {
            gx = ServerSettings.startingGx;
            gy = ServerSettings.startingGy;
        }
        else
        {   
            ClearStartLocations(WorldMap.world.berryBushes);
            ClearStartLocations(WorldMap.world.bananaPlants);
            ClearStartLocations(WorldMap.world.wildCarrots);
            ClearStartLocations(WorldMap.world.cactuses);
            ClearStartLocations(WorldMap.world.wildGarlics);

            var berryBushes = [for(obj in WorldMap.world.berryBushes) obj];
            var bananaPlants = [for(obj in WorldMap.world.bananaPlants) obj];
            var wildCarrots = [for(obj in WorldMap.world.wildCarrots) obj];
            var cactuses = [for(obj in WorldMap.world.cactuses) obj];
            var wildGarlics = [for(obj in WorldMap.world.wildGarlics) obj];
            var rand = WorldMap.calculateRandomInt(4); 
            var world = WorldMap.world;
            var startLocations = berryBushes;

            trace('spawnAsEve: bananaPlants: ${bananaPlantsTmp.length} ==> ${bananaPlants.length} berryBushes: ${berryBushesTmp.length} ==> ${berryBushes.length}');
            trace('spawnAsEve: wildCarrots: ${wildCarrotsTmp.length} ==> ${wildCarrots.length} cactuses: ${cactusesTmp.length} ==> ${cactuses.length}');
            trace('spawnAsEve: rand: $rand wildGarlics: ${wildGarlicsTmp.length} ==> ${wildGarlics.length}');

            //var startLocations = rand > 0.5 && bananaPlants.length > 10 ? bananaPlants : berryBushes;
            
            if(rand == 1 && bananaPlants.length > 5) startLocations = bananaPlants;
            else if(rand == 2 && bananaPlants.length > 5) startLocations = wildCarrots;
            else if(rand == 3 && bananaPlants.length > 5) startLocations = cactuses;
            else if(rand == 4 && bananaPlants.length > 5) startLocations = wildGarlics;

            var bestLocation = null;
            var bestLocationFitness = -99999999999999999999999;
            // 762 Flowering Barrel Cactus / 763 Fruiting Barrel Cactus / 404 Wild Carrot
            //var foodArray = [30, 2142, 36, 761, 4251, 762, 763, 404];           

            // TODO in a bigger map this might spread spawns too far
            // TODO consider deadly animals
            for(i in 0...20)
            {
                var location = startLocations[WorldMap.calculateRandomInt(startLocations.length - 1)];
                location = world.getObjectHelper(location.tx, location.ty);                
                var fitness = 1 + location.numberOfUses;
                var sumDistHumans:Float = 1;

                // consider graves
                var hasCloseBlockingGrave = this.account.hasCloseBlockingGrave(location.tx, location.ty);
                var hasCloseNonBlockingGrave = this.account.hasCloseNonBlockingGrave(location.tx, location.ty) ;
                sumDistHumans += hasCloseBlockingGrave ? 5 : 0; 
                fitness += hasCloseNonBlockingGrave ? 5 : 0;

                for(p in GlobalPlayerInstance.AllPlayers)
                {
                    var quadDist = 1 + AiHelper.CalculateDistanceToObject(p, location);
                    sumDistHumans += (10000 / quadDist); // 100 tiles
                }   

                var totalFitness = fitness / sumDistHumans;
                trace('spawnAsEve: fitness: $fitness / sumDistHumans: $sumDistHumans = $totalFitness closeBadGrave: ${hasCloseBlockingGrave} closeGoodGrave: ${hasCloseNonBlockingGrave}');
                if(totalFitness < bestLocationFitness) continue;

                bestLocation = location;
                bestLocationFitness = totalFitness;

            }

            if(bestLocation == null)
            {
                gx = ServerSettings.startingGx;
                gy = ServerSettings.startingGy;
            }
            else
            {
                gx = bestLocation.tx;
                gy = bestLocation.ty;
            }         
        }

        if(lastEveOrAdam == null)
        {
            lastEveOrAdam = this;
            this.lineage.myEveId = this.p_id;

            // give eve the right color fitting to closest special biome
            var closeSpecialBiomePersonColor = getCloseSpecialBiomePersonColor(this.tx, this.ty);
            if(closeSpecialBiomePersonColor > 0)
            {
                var female = ServerSettings.ChanceForFemaleChild >= 0.5;
                var personsByColor = female ? ObjectData.femaleByRaceObjectData : ObjectData.maleByRaceObjectData;
                var persons = personsByColor[closeSpecialBiomePersonColor];
                setObjectId(persons[WorldMap.calculateRandomInt(persons.length-1)].id); 

                trace('New player id: ${this.p_id} is an EVE / ADAM with color: ${this.getColor()} as ${this.lineage.className}');
            }
        }
        else
        {
            this.lineage.myEveId = lastEveOrAdam.p_id;
            // Spawn An Eve / Adam is to last Eve / Adam
            this.followPlayer = lastEveOrAdam;
            //lastEveOrAdam.followPlayer = this;
            this.mother = lastEveOrAdam; // its not really the mother, but its the mother in spirit...  

            gx = lastEveOrAdam.tx;
            gy = lastEveOrAdam.ty;

            var female = lastEveOrAdam.isFemale() == false;
            var personsByColor = female ? ObjectData.femaleByRaceObjectData : ObjectData.maleByRaceObjectData;
            var persons = personsByColor[lastEveOrAdam.getColor()];
            setObjectId(persons[WorldMap.calculateRandomInt(persons.length-1)].id); 

            lastEveOrAdam = null;

            trace('An Eve / Adam id: ${this.p_id} is born to an Eve / Adam with color: ${this.getColor()} as ${this.lineage.className}');
        }

        name = isFemale() ? "EVE" : "ADAM";

        if(isAi) lastAiEveOrAdam = lastEveOrAdam;
        else lastHumanEveOrAdam = lastEveOrAdam;
    } 

    private static function ClearStartLocations(locations:Map<Int,ObjectHelper>)
    {
        var foodArray = [30, 2142, 36, 761, 4251, 762, 763, 404];

        for(key in locations.keys())
        {
            var obj = locations[key];

            if(foodArray.contains(obj.parentId) == false)
            {
                locations.remove(key);                                
            }
        }
    }

    // TODO higher change of children for smaler families
    // TODO spawn noobs more likely noble
    // TODO consider past families of player
    private function spawnAsChild() : Bool
    {
        trace('Spawn As Child: ${this.p_id} ${this.account.email}');

        var mother = GetFittestMother(this);     
        if(mother == null) return false;

        // TODO father
        this.lineage.myEveId = mother.lineage.myEveId;
        this.mother = mother;
        this.followPlayer = mother; // the mother is the leader

        // TODO consider dead children for mother fitness
        mother.exhaustion += ServerSettings.NewChildExhaustionForMother;
        mother.childrenBirthMali += 1; // make it less likely to get new child
        if(mother.mother != null) mother.mother.childrenBirthMali += 0.5; // make it less likely to get new child for each grandkid

        this.age = 0.01;
        this.trueAge = 0.01;
        gx = mother.tx;
        gy = mother.ty;

        var motherColor = mother.getColor();
        var color = motherColor;
        var female = ServerSettings.ChanceForFemaleChild > WorldMap.calculateRandomFloat(); 
        var personsByColor = female ? ObjectData.femaleByRaceObjectData : ObjectData.maleByRaceObjectData;
        var rand = WorldMap.calculateRandomFloat();
        var closeSpecialBiomePersonColor = getCloseSpecialBiomePersonColor(this.tx, this.ty);
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
        setObjectId(persons[WorldMap.calculateRandomInt(persons.length-1)].id); 
        
        trace('New child is born to mother: ${mother.name} ${mother.familyName} female: $female motherColor: $motherColor childColor: ${this.getColor()}');

        InheritEatenFoodCounts(this);
        
        return true;
    } 

    /**Girls inherit food count from father boys from mother.**/
    private static function InheritEatenFoodCounts(child:GlobalPlayerInstance)
    {
        var fromPlayer = child.mother.mother == null ? child.mother : child.mother.mother;
        if(child.father != null && child.isFemale()) fromPlayer = child.father.father == null ? child.father : child.father.father;
        var inheritFromGrandparents = fromPlayer == child.mother.mother || (child.father != null && fromPlayer == child.father.father);

        for(food in fromPlayer.hasEatenMap.keys())
        {
            var foodCount = fromPlayer.hasEatenMap[food]; 
            if(foodCount > 0)
            {
                foodCount -= ServerSettings.HasEatenReductionForNextGeneration;
                if(foodCount < 0) foodCount = 0;
            }
            if(foodCount > ServerSettings.MaxHasEatenForNextGeneration) foodCount = ServerSettings.MaxHasEatenForNextGeneration;
            child.hasEatenMap[food] = foodCount;

            trace('Inherit from: ${fromPlayer.name} GP?: $inheritFromGrandparents food: ${GetName(food)} value: $foodCount');
        }
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
    // Snow --> Ginger / Grey --> White / Jungle --> Brown / Desert --> Black
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

    private static function GetFittestMother(child:GlobalPlayerInstance) : GlobalPlayerInstance
    {
        var mother:GlobalPlayerInstance = null;
        var fitness = -1000.0;

        //var count = 0;
        //for(p in AllPlayers) count++;

        //trace('Spawn As Child: GetFittestMother ${child.account.email} count: $count');

        // search fertile mother
        for (p in AllPlayers)
        {            
            var tmpFitness = CalculateMotherFitness(p, child);            

            trace('Spawn As Child: ${child.account.email} Fitness: ${Math.round(tmpFitness*10) / 10} ${p.name} ${p.familyName}');

            if(tmpFitness < -100) continue;

            if(tmpFitness > fitness || mother == null)
            {
                mother = p;
                fitness = tmpFitness;    
            }
        }

        return mother;
    }

    // TODO test
    private function calculateClassBoni(child:GlobalPlayerInstance) : Float
    {
        var childClass:PrestigeClass = child.lineage.prestigeClass;
        var motherClass = this.lineage.prestigeClass;

        if(motherClass == childClass) return 2;
        if(motherClass == PrestigeClass.Noble && childClass == PrestigeClass.Serf) return -3;
        if(motherClass == PrestigeClass.Serf && childClass == PrestigeClass.Noble) return -3;

        return 0;
    }

    private static function CalculateMotherFitness(p:GlobalPlayerInstance, child:GlobalPlayerInstance) : Float
    {        
        var childIsHuman = child.isAi() == false;
        var motherIsHuman = p.isAi() == false;
        var maxExhaustion = childIsHuman == motherIsHuman ? 10 : 5;

        if(p.deleted) return -1000;
        if(p.isFertile() == false) return -1000;
        if(p.isWounded()) return -1000;
        if(p.food_store < 0) return -1000; // no starving mothers
        if(p.exhaustion > maxExhaustion) return -1000; // no super exhausted mothers
        
        // boni
        var tmpFitness = 0.0;
        tmpFitness += p.food_store / 10; // the more food the more likely 
        tmpFitness += p.yum_bonus / 10; // the more food the more likely 
        tmpFitness += p.food_store_max / 10; // the more healthy the more likely 
        tmpFitness += p.calculateClassBoni(child); // the closer the mother is to same class the better
        tmpFitness += child.account.hasCloseNonBlockingGrave(p.tx, p.ty) ? 3 : 0;
        //tmpFitness += p.yum_multiplier / 20; // the more yum / prestige the more likely  // not needed since influencing food_store_max

        // mali
        var temperatureMail = Math.pow(((p.heat - 0.5) * 10), 2) / 10; // between 0 and 2.5 for very bad temperature
        tmpFitness -= temperatureMail;
        tmpFitness -= p.exhaustion / 5;
        tmpFitness -= p.childrenBirthMali; // the more children the less likely
        tmpFitness -= child.account.hasCloseBlockingGrave(p.tx, p.ty) ? 10 : 0; // make less likely to incarnate if there is a blocking grave close by
        tmpFitness -= p.heldObject.objectData.speedMult > 1.1 ? 1 : 0; // if player is using fast objects
        tmpFitness -= p.heldObject.id != 0 ? 1 : 0; // if player is holding objects
        tmpFitness -= motherIsHuman && child.isAi() ? ServerSettings.HumanMotherBirthMaliForAiChild : 0;
        tmpFitness -= p.isAi() && childIsHuman ? ServerSettings.AiMotherBirthMaliForHumanChild : 0;
        
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

    public override function isMoving()
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
        Connection.SendEmoteToAll(this, id);
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
            return dropPlayer(x, y);
        }
    }

    // TODO better use relative toData which transforms x,y to relative position
    public function toRelativeData(forPlayer:PlayerInstance):String
    {
        var heldObject = o_id[0] < 0 ?  '${o_id[0]}' : MapData.stringID(o_id);

        return toData(
            tx - forPlayer.gx,
            ty - forPlayer.gy,
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
        var targetX = player.tx - this.gx;
        var targetY = player.ty - this.gy;

        return isClose(targetX,targetY,distance);
    }

    /** works with coordinates relative to the player **/ //TODO does not consider round map
    public function isClose(x:Int, y:Int, distance:Int = 1):Bool
    {    
        var xDiff = this.x - x;
        var yDiff = this.y - y;

        return (xDiff * xDiff + yDiff * yDiff <= distance * distance);
    }

    public function isCloseToPlayerUseExact(target:GlobalPlayerInstance, distance:Float = 1):Bool
    {    
        return this.moveHelper.isCloseToPlayerUseExact(target, distance);
    }

    public function isCloseUseExact(targetTx:Float, targetTy:Float, distance:Float = 1) : Bool
    {    
        return this.moveHelper.isCloseUseExact(targetTx, targetTy, distance);
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

    public function CalculateHealthSpeedFactor() : Float
    {
        return CalculateHealthFactor(1.2, 0.8);
    }

    public function CalculateHealthAgeFactor() : Float
    {
        return CalculateHealthFactor(2, 0.5);
    }

    public function CalculateSpeedMaxFoodStoreFactor() : Float
    {
        return CalculateHealthFactor(1.5, 0.5);
    }

    public function CalculateHealthFactor(maxBoni:Float, maxMali:Float) : Float
    {
        var health:Float = this.yum_multiplier; 
        var healthFactor:Float; 
        var medianHealth = medianPrestige;

        health -= medianHealth * (this.trueAge / 30); // at half of the life the median medianHealth should be reached

        // healthFactor 1.13 if health double ServerSettings.HealthFactor
        if(health >= 0) healthFactor = (maxBoni  * health + medianHealth) / (health + medianHealth); 
        else healthFactor = (health - medianHealth) / ( (1 / maxMali) * health - medianHealth);

        //trace('HEALTH: maxBoni: $maxBoni maxMali: $maxMali health: $health medianHealth: $medianHealth healthFactor: $healthFactor');

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
    public function say(text:String, toSelf:Bool = false)
    {
        AllPlayerMutex.acquire();

        Macro.exception(sayHelper(text, toSelf));

        AllPlayerMutex.release();
    }
    
    public function sayHelper(text:String, toSelf:Bool = false)
    {
        //trace('say: $text');

        var player = this;
        var curse = 0;
        var id = player.p_id;

        text = text.toUpperCase();

        if(toSelf)
        {
            text = '?{$text}?';
            this.connection.send(PLAYER_SAYS,['$id/$curse $text']);
            this.connection.send(FRAME);
            return;
        }

        if(StringTools.contains(text, '!'))
        {
            if(ServerSettings.AllowDebugCommmands) DoDebugCommands(player, text);
        }

        if(lastSayInSec > 0 && ServerSettings.debug == false) return;
        lastSayInSec = 1;

        var maxLenght = player.age < 10 ? Math.ceil(player.age * 2) : player.age < 20 ? Math.ceil(player.age * 4) : 80; 

        if(text.startsWith('/') == false &&  text.length > maxLenght) text = text.substr(0, maxLenght);

        text = NamingHelper.DoNaming(this, text);

        this.lineage.lastSaid = text;

        if(doCommands(text))
        {
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
    }

    public function exile(target:GlobalPlayerInstance, messageIfAllreadyExiled:Bool = true) : Bool
    {
        if(target == null) return false;

        if(target.exiledByPlayers.exists(this.p_id))
        {
            if(messageIfAllreadyExiled) this.connection.sendGlobalMessage('${target.name} is allready exiled');
            return  false;
        } 

        var leader = target.getTopLeader();

        target.exiledByPlayers[this.p_id] = this;

        this.connection.sendGlobalMessage('YOU_EXILED:_${target.name}_${target.familyName}');
        if(leader != target.getTopLeader()) target.connection.sendGlobalMessage('YOU_HAVE_BEEN_EXILED_BY:_${this.name}_${this.familyName} YOU CAN BE LEGALLY KILLED!');

        Connection.SendExileToAll(this, target);

        this.doEmote(Emote.angry);

        return true;
    }

    public function redeem(target:GlobalPlayerInstance) : Bool
    {
        if(target == null) return false;

        // TODO target may be exiled by a sub leader, in case so redeem him also? 
        if(target.exiledByPlayers.exists(target.p_id) == false)
        {
            this.connection.sendGlobalMessage('Cannot redeem ${target.name} if not exiled first!');
            return false;
        }

        target.exiledByPlayers.remove(target.p_id);

        Connection.SendFullExileListToAll(target);

        this.connection.sendGlobalMessage('YOU_REDEEM:_${target.name}_${target.familyName}');
        target.connection.sendGlobalMessage('YOU_HAVE_BEEN_REDEEMED_BY:_${this.name}_${this.familyName}');

        this.doEmote(Emote.happy);
        target.doEmote(Emote.happy);

        return true;
    }

    public function isExiledBy(player:GlobalPlayerInstance) : Bool
    {
        return this.exiledByPlayers.exists(player.p_id);
    }

    public function isExiledByAnyLeader(player:GlobalPlayerInstance) : GlobalPlayerInstance
    {
        if(this.isExiledBy(player)) return player;

        var topLeader = player.getTopLeader();
        
        if(this.isExiledBy(topLeader)) return topLeader;

        return null;
    }

    private function doCommands(message:String) : Bool
    {
        var name = NamingHelper.GetName(message);

        if(message.startsWith('I EXILE '))
        {
            var target = NamingHelper.GetPlayerByNameWithMessage(this, name);
            return this.exile(target);
        }

        if(message.startsWith('I REDEEM '))
        {
            var target = NamingHelper.GetPlayerByNameWithMessage(this, name);
            return this.redeem(target);
        }

        if(message.startsWith('I FOLLOW '))
        {
            if(name == "ME")
            {
                // TODO check if follower color changes to new color or if needed to be send again

                this.followPlayer = null;

                this.connection.sendGlobalMessage('YOU_FOLLOW_NOW_NO_ONE!');

                Connection.SendFollowingToAll(this);

                this.doEmote(Emote.happy);

                return true;
            }

            var player = NamingHelper.GetPlayerByName(this, name);
            
            if(player == null || player == this)
            {
                this.connection.sendGlobalMessage('No one found close enough with the name ${name}!');
                return  false;
            } 

            var exileLeader = this.isExiledByAnyLeader(player);

            if(exileLeader != null)
            {
                this.connection.sendGlobalMessage('${exileLeader.name} has exiled you already!');
                return  false;
            } 

            if(player == this.followPlayer)
            {
                this.connection.sendGlobalMessage('You follow allready ${player.name}!');
                return  false;
            } 

            var tmpFollow = this.followPlayer;
            this.followPlayer = player;
            var leader = this.getTopLeader();

            // TODO allow other leader through follow?
            if(leader == null)
            {
                //trace('FOLLOW: CIRCULAR FOLLOW --> NO CHANGE');
                this.followPlayer = tmpFollow;

                this.connection.sendGlobalMessage('${player.name} is following you or one of your allies!');
                
                return false;
            }

            this.followPlayer = tmpFollow;

            if(leader.newFollower != null)
            {
                var time = Math.ceil(leader.newFollowerTime);

                if(leader.newFollower == this) this.connection.sendGlobalMessage('Leader ${leader.name} will accept you in ${time} seconds...');
                else this.connection.sendGlobalMessage('Top leader ${leader.name} is considering some one else. Try in ${time} seconds...');

                return false;
            }

            if(player.newFollower != null)
            {
                var time = Math.ceil(player.newFollowerTime);

                this.connection.sendGlobalMessage('${player.name} is considering some one else. Try in ${time} seconds...');

                return false;
            }

            leader.newFollower = this;
            leader.newFollowerFor = player;
            leader.newFollowerTime = ServerSettings.TimeConfirmNewFollower;

            // since new leader might not be the top leader
            player.newFollower = this;
            player.newFollowerFor = player;
            player.newFollowerTime = ServerSettings.TimeConfirmNewFollower;

            this.connection.sendGlobalMessage('In ${leader.newFollowerTime} seconds you follow ${player.name}_${player.familyName}');

            //Connection.SendFollowingToAll(this);

            // inform leader
            leader.connection.sendMapLocation(leader, 'FOLLOWER', 'follower');
            leader.connection.sendGlobalMessage('YOU_HAVE_A_NEW_FOLLOWER:_${this.name}_${this.familyName}');
            leader.doEmote(Emote.hubba);

            if(leader != player)
            {
                player.connection.sendMapLocation(player, 'FOLLOWER', 'follower');
                player.connection.sendGlobalMessage('YOU_HAVE_A_NEW_FOLLOWER:_${this.name}_${this.familyName}');
                player.doEmote(Emote.hubba);
            }

            this.doEmote(Emote.happy);
            
            return true;
        }

        if(message.startsWith('ORDER, '))
        {
            message = message.replace('ORDER, ', '');

            this.connection.sendGlobalMessage('ORDER:_$message');

            for(c in Connection.getConnections())
            {
                var leader = c.player.getTopLeader();
                if(leader == this) c.sendGlobalMessage('ORDER:_$message');

                this.doEmote(Emote.biomeRelief);                
            }
            // TODO AI
            return true;
        }

        if(message.startsWith('I GIVE '))
        {
            var target = NamingHelper.GetPlayerByName(this, name);

            if(target == null || target == this)
            {
                this.connection.sendGlobalMessage('No one found close enough with the name ${name}!');
                return  false;
            } 

            var strings = message.split(' ');

            if(strings.length < 4) return false;

            var coinText = strings[3];
            var amount = 0;

            for(ii in 0...coinText.length)
            {
                if(coinText.charAt(ii) == 'I') amount += 1;
                else if(coinText.charAt(ii) == 'V') amount += 5;
                else if(coinText.charAt(ii) == 'X') amount += 10;
                else if(coinText.charAt(ii) == 'L') amount += 50;
                else if(coinText.charAt(ii) == 'C') amount += 100;
                else if(coinText.charAt(ii) == 'D') amount += 500;
                else if(coinText.charAt(ii) == 'M') amount += 1000;
            }

            if(this.coins < amount)
            {
                this.connection.sendGlobalMessage('YOU_NEED_${amount}_COINS(S)._BUT_YOU_HAVE_${this.coins}!');

                return false;
            }

            this.coins -= amount;
            target.coins += amount;

            this.connection.sendGlobalMessage('YOU_GAVE_${target.name}_${target.familyName}_${amount}_COINS(S)._YOU_HAVE_NOW_${Math.ceil(this.coins)}!');
            target.connection.sendGlobalMessage('${this.name}_${this.familyName} GAVE YOU ${amount}_COINS(S)._YOU_HAVE_NOW_${Math.ceil(target.coins)}!');

            this.doEmote(Emote.happy); 

            trace('coinText: $coinText amount: $amount');

            return true;
        }

        if(message.contains('OWNES THIS') ||  message.contains('OWN THIS'))
        {
            name = NamingHelper.GetName(message, true);

            var target = NamingHelper.GetPlayerByName(this, name);

            //trace('Owner: ${name}');

            if(target == null || target == this)
            {
                this.connection.sendGlobalMessage('No one found close enough with the name ${name}!');
                return  false;
            }

            //trace('Owner: target ${target.name}');

            var obj = AiHelper.GetClosestObjectOwnedByPlayer(this);

            if(obj == null)
            {
                this.connection.sendGlobalMessage('No close enough property that you own found!');
                return  false;
            }

            //trace('Owner: ${obj.description}');

            if(obj.isOwnedByPlayer(target))
            {
                this.connection.sendGlobalMessage('${target.name} ownes this allready!');
                return  false;
            }

            obj.addOwner(target);

            target.owning.push(obj);

            target.connection.sendGlobalMessage('${this.name} gave you a new property!'); // TODO pointer

            this.doEmote(Emote.happy);  

            return true;
        }

        return true;
    }

    // if people follow circular outcome is null / max 10 deep hierarchy is supported
    public function getTopLeader(stopWithPlayer:GlobalPlayerInstance = null) : GlobalPlayerInstance
    {
        //trace('getTopLeader0 ${this.name}');

        if(this.followPlayer == null) return this; // is his own leader

        var lastLeader = this;
        var leader = this.followPlayer;

        for(ii in 0...10)
        {
            if(leader.isDeleted()) return lastLeader; // TODO check why no new leader was chosen
            //trace('getTopLeader1 ${lastLeader.name} --> ${leader.name}');
            if(this.exiledByPlayers.exists(leader.p_id)) return lastLeader; // is exiled by leader
            //trace('getTopLeader2 ${lastLeader.name} --> ${leader.name} ' + leader.exiledByPlayers);
            if(leader.exiledByPlayers.exists(this.p_id)) return lastLeader; // player exiled leader // still ally in this case???
            //trace('getTopLeader3 ${lastLeader.name} --> ${leader.name}');
            if(leader.followPlayer == null) return leader;

            if(leader == stopWithPlayer) return leader;

            lastLeader = leader;
            leader = leader.followPlayer;
        }

        return null;
    }

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

        AllPlayerMutex.acquire();

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

        AllPlayerMutex.release();
    }
    
    public function move(x:Int,y:Int,seq:Int, moves:Array<Pos>)
    {
        MoveHelper.move(this, x, y, seq, moves);
    }

    private function doSelf(x:Int, y:Int, clothingSlot:Int) : Bool
    {
        //trace('self: ${this.o_id[0]} ${heldObject.objectData.description} clothingSlot: $clothingSlot');

        if(this.o_id[0] < 0) return false; // is holding player

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
        AllPlayerMutex.acquire();
        
        var done = false;
        Macro.exception(done = doOnOtherHelper(x,y,clothingSlot, playerId));                
        if(done == false)
        {
            // send always PU so that player wont get stuck
            this.connection.send(PLAYER_UPDATE,[this.toData()]);
            this.connection.send(FRAME);
        }

        AllPlayerMutex.release();
        
        return done;
    }

    //public function doOnOtherHelper(x:Int, y:Int, clothingSlot:Int, targetPlayer:GlobalPlayerInstance) : Bool
    public function doOnOtherHelper(x:Int, y:Int, clothingSlot:Int, playerId:Int, ?infos:haxe.PosInfos) : Bool
    {
        if(ServerSettings.DebugPlayer) trace('doOnOtherHelper: playerId: ${playerId} ${this.o_id[0]} ${heldObject.objectData.description} clothingSlot: $clothingSlot');

        if(this.o_id[0] < 0) return false; // is holding player
        // 838 Dont feed dam drugs! Wormless Soil Pit with Mushroom // 837 Psilocybe Mushroom
        if(heldObject.objectData.isDrugs()) return false;        

        var targetPlayer = getPlayerAt(x,y, playerId);
        if(targetPlayer == null)
        {
            
            trace('doOnOtherHelper: could not find target player! ${infos.methodName}');
            //throw new Exception('');
            return false;
        }
    
        if(this.isClose(targetPlayer.tx - this.gx , targetPlayer.ty - this.gy) == false)
        {
            trace('doOnOtherHelper: Target position is too far away player: ${this.tx},${this.ty} target: ${targetPlayer.tx},${targetPlayer.ty}');
            return false; 
        }

        if(clothingSlot < 0)
        {
            if(doEating(this, targetPlayer)) return true;
        }

        if(doSwitchCloths(this, targetPlayer, clothingSlot)) return true;

        if(targetPlayer.isWounded())
        {
            var trans = TransitionImporter.GetTrans(this.heldObject, targetPlayer.heldObject);

            if(trans != null)
            {
                //trace('HEALING: ' + trans.getDesciption());

                var objTo = targetPlayer.heldObject;
                var alternativeTimeOutcome = objTo.objectData.alternativeTimeOutcome; 
                objTo.id = alternativeTimeOutcome >=0 ? alternativeTimeOutcome : trans.newTargetID;
                objTo.creationTimeInTicks = TimeHelper.tick;
                targetPlayer.hiddenWound = null; // if there is a new hidden wound setHeldObject will set it
                targetPlayer.setHeldObject(objTo);
                targetPlayer.setHeldObjectOriginNotValid(); // no animation

                var objFrom = this.heldObject;
                objFrom.objectData = ObjectData.getObjectData(trans.newActorID);
                objFrom.creationTimeInTicks = TimeHelper.tick;
                this.setHeldObject(objFrom);
                this.setHeldObjectOriginNotValid(); // no animation

                Connection.SendEmoteToAll(this, Emote.happy);
                Connection.SendEmoteToAll(targetPlayer, Emote.happy);

                // TODO alsow fix below in doing transitions. How does it work?
                // TODO fix Needle and Thread --> Bone Needle 192 --> 191
                // TODO fix Needle and Ball of Thread 1126 --> Tool use

                return true;
            }
        }

        return false;
    }

    public function getPlayerAt(x:Int, y:Int, playerId:Int) : GlobalPlayerInstance
    {
        return GetPlayerAt(x,y,playerId);
    }

    public static function GetPlayerAt(x:Int, y:Int, playerId:Int) : GlobalPlayerInstance
    {
        for(player in GlobalPlayerInstance.AllPlayers)
        {
            if(player.deleted) continue;

            if(player.p_id == playerId) return player;

            if(playerId <= 0)
            {
                if(player.x == x && player.y == y) return player;
            }
        }

        return null;
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

            var pX = c.player.tx - this.gx;
            var pY = c.player.ty - this.gy;
            var tmpDistance = (pX - x) * (pX - x) + (pY - y) * (pY - y);

            if(tmpDistance < distance) player = c.player;
        }

        if(onlyHuman) return player;

        for(ai in Connection.getAis())
        {
            if(ai.player.deleted) continue;

            if(ai.player == this) continue;
            
            var pX = ai.player.tx - this.gx;
            var pY = ai.player.ty - this.gy;
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
        if(playerFrom.o_id[0] == 0) return false;
        if(playerFrom.o_id[0] < 0) return false; // is holding player

        if(playerFrom.age < ServerSettings.MinAgeToEat)
        {
            trace('too young to eat: age: ${Math.round(playerFrom.age)} < MinAgeToEat: ${ServerSettings.MinAgeToEat} ');
            return false;
        }

        if(ServerSettings.AllowSelfEatingIfIll == false && playerTo.hasYellowFever() && playerFrom == playerTo)
        {
            trace('is ill cannot feed himself!');
            playerTo.doEmote(Emote.refuseFood);
            return false;
        }

        var heldObjData = playerFrom.heldObject.objectData;
        if(heldObjData.dummyParent != null) heldObjData = heldObjData.dummyParent;

        var foodValue:Float = heldObjData.foodValue;

        if(ServerSettings.DebugEating) trace('FOOD: food_store_max: ${playerTo.food_store_max} food_store: ${playerTo.food_store} foodValue: ${foodValue}');

        if(foodValue < 1)
        {
            trace('cannot eat this stuff no food value!!! ${heldObjData.description}');
            return false;
        }
        
        if(playerTo.food_store_max - playerTo.food_store < Math.ceil(foodValue / 4))
        {
            trace('too full to eat: food_store_max: ${playerTo.food_store_max} - food_store: ${playerTo.food_store} < foodValue: $foodValue  / 3');
            playerTo.doEmote(Emote.refuseFood);
            return false;
        }

        var countEaten = playerTo.hasEatenMap[heldObjData.id]; 
        if(countEaten < 0) countEaten = 0;    

        foodValue += ServerSettings.YumBonus;
        foodValue -= countEaten;
        var isFoodYum = countEaten < ServerSettings.YumBonus;  //playerFrom.isHoldingYum();

        var isCravingEatenObject = heldObjData.id == playerTo.currentlyCraving;
        if(isCravingEatenObject) foodValue += 1; // craved food give more boni

        var isSuperMeh = foodValue < playerFrom.heldObject.objectData.foodValue / 2;

        if(isSuperMeh) foodValue = playerFrom.heldObject.objectData.foodValue / 2;

        if(isSuperMeh && playerTo.food_store > 2)
        {
            trace('Supermeh food can only be eaten if starving to death: foodValue: $foodValue original food value: ${playerFrom.heldObject.objectData.foodValue} food_store: ${playerTo.food_store}');
            if(playerTo == playerFrom) playerTo.doEmote(Emote.ill);
            else playerFrom.doEmote(Emote.sad);
            return false;
        }
        if(playerTo != playerFrom && isFoodYum == false && playerTo.food_store > 2)
        {
            trace('Other player can only feed YUM if not starving to death');
            playerFrom.doEmote(Emote.sad);
            return false;
        }

        var foodEaten:Float = ServerSettings.FoodReductionPerEating;

        if(isSuperMeh == false)
        {
            var countEaten = playerTo.hasEatenMap[heldObjData.id]; 

            if(countEaten < 0)
            {
                foodEaten = Math.max(1, Math.floor(-countEaten / 2)); // eat more if it its a craving
                playerTo.hasEatenMap[heldObjData.id] += foodEaten;
                if(foodEaten > 1) foodEaten = 1 + (foodEaten - 1) / 2; 

                if(ServerSettings.DebugEating) trace('Craving: foodEaten: $foodEaten countEaten: $countEaten --> ${playerTo.hasEatenMap[heldObjData.id]}');
            }
            else
            {
                if(isFoodYum == false) foodEaten *= ServerSettings.FoodReductionFaktorForEatingMeh;
                playerTo.hasEatenMap[heldObjData.id] += foodEaten;
                if(ServerSettings.DebugEating) trace('No Craving: foodEaten: $foodEaten countEaten: $countEaten --> ${playerTo.hasEatenMap[heldObjData.id]}');
            }

            playerTo.doIncreaseFoodValue(heldObjData.id, foodEaten);
            //playerTo.say('FC ${playerTo.hasEatenMap[heldObjData.id]}');
        }

        // eating YUM increases prestige / score while eating MEH reduces it
        if(isFoodYum)
        {
            if(countEaten < 0)
            {
                var gainedPrestige = isCravingEatenObject ? foodEaten + 1 : foodEaten;
                playerTo.addHealthAndPrestige(gainedPrestige);
                if(playerFrom != playerTo) playerFrom.addHealthAndPrestige(gainedPrestige * 0.2);
            }
            else
            {
                playerTo.addHealthAndPrestige(1);
                if(playerFrom != playerTo)playerFrom.addHealthAndPrestige(0.2);          
            }
        }
        else
        {
            if(isSuperMeh)
                playerTo.addHealthAndPrestige(ServerSettings.HealthLostWhenEatingSuperMeh);
            else 
                playerTo.addHealthAndPrestige(ServerSettings.HealthLostWhenEatingMeh);
            //if(playerFrom != playerTo) playerFrom.yum_multiplier += 0.5; // saved one from starving to death
        }

        if(heldObjData.isDrugs()) // eating lovely mushrooms give protection against fever 
        {
            playerTo.yellowfeverCount += ServerSettings.ResistenceAginstFeverForEatingMushrooms;
        }
            
        if(ServerSettings.DebugEating) trace('YUM: ${heldObjData.description} foodValue: $foodValue countEaten: $countEaten');

        // food_store food_capacity last_ate_id last_ate_fill_max move_speed responsible_id
       
        playerTo.last_ate_fill_max = Math.ceil(playerTo.food_store);
        if(ServerSettings.DebugEating) trace('last_ate_fill_max: ${playerTo.last_ate_fill_max}');
        //this.food_store += foodValue;
        playerTo.just_ate = 1;
        playerTo.last_ate_id = heldObjData.id;
        playerTo.responsible_id = playerFrom.p_id; // -1; // self???
        //this.o_transition_source_id = -1;

        playerTo.addFood(foodValue);

        playerTo.move_speed = MoveHelper.calculateSpeed(playerTo, playerTo.tx, playerTo.ty);

        playerTo.sendFoodUpdate();

        // check if there is a player transition like:
        // 2143 + -1 = 2144 + 0 Banana
        // 1251 + -1 = 1251 + 0 lastUseActor: false Bowl of Stew
        // 1251 + -1 = 235 + 0 lastUseActor: true Bowl of Stew
        if(TransitionHelper.DoChangeNumberOfUsesOnActor(playerFrom.heldObject, false, false) == false)
        {
            if(ServerSettings.DebugEating) trace('FOOD: set held object null');
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
        else if(isFoodYum) playerTo.doEmote(Emote.happy);
        else if(isSuperMeh) playerTo.doEmote(Emote.ill); 
        else playerTo.doEmote(Emote.sad);
        
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
        var isHoldingPlayer = this.o_id[0] < 0;

        if(isHoldingPlayer && this.heldPlayer == null) throw new Exception('Is holding player but no player set!');
        if(isHoldingPlayer) isHoldingPlayer = false; // TODO?

        player.forced = false;
        player.action = 1;        
        player.o_id = this.heldPlayer != null ? this.o_id = [-heldPlayer.p_id] : this.heldObject.toArray();

        //player.o_transition_source_id = this.newTransitionSource; TODO ??????????????????????????
        player.o_transition_source_id = isHoldingPlayer || objOriginValid ? -1 : this.heldObject.id;

        // this changes where the client moves the object from on display
        player.o_origin_x = objOriginValid ? x : 0;
        player.o_origin_y = objOriginValid ? y : 0;
        
        player.o_origin_valid = objOriginValid ? 1 : 0; // if set to 0 no animation is displayed to pick up hold obj from o_origin_x o_origin_y
        
        player.action_target_x = x;
        player.action_target_y = y;
    }

    public function setHeldObjectOriginNotValid()
    {
        var player = this;
        var isHoldingPlayer = this.o_id[0] < 0;

        player.o_transition_source_id = isHoldingPlayer ? -1 : player.heldObject.id;

        // this changes where the client moves the object from on display
        player.o_origin_x = 0;
        player.o_origin_y = 0;

        player.action = 1;   
        player.action_target_x = 0;
        player.action_target_y = 0;
        
        player.o_origin_valid = 0; // if set to 0 no animation is displayed to pick up hold obj from o_origin_x o_origin_y
    }

    /*
    CR
    food_id bonus
    #

    Tells player about which food they're currently craving, and how much their
    YUM multiplier will increase when they eat it.
    */

    private function doIncreaseFoodValue(eatenFoodId:Int, amountEaten:Float)
    {
        //trace('${this.name} IncreaseFoodValue: ${eatenFoodId}');
        
        if(hasEatenMap[eatenFoodId] > 0) cravings.remove(eatenFoodId);

        var hasEatenKeys = [for(key in hasEatenMap.keys()) key];

        if(ServerSettings.DebugEating) trace('${this.name} IncreaseFoodValue: hasEatenKeys.length: ${hasEatenKeys.length}');

        if(hasEatenKeys.length < 1) return;

        var random = WorldMap.calculateRandomInt(hasEatenKeys.length -1);
        var key = hasEatenKeys[random];

        //if(ServerSettings.DebugEating) trace('IncreaseFoodValue: random: $random hasEatenKeys.length: ${hasEatenKeys.length}');

        var newHasEatenCount = hasEatenMap[key];
        var cravingHasEatenCount = hasEatenMap[currentlyCraving];
        
        if(key != eatenFoodId && WorldMap.calculateRandomFloat() < ServerSettings.YumFoodRestore)
        {
            restoreFoodCount(key, amountEaten);
            newHasEatenCount = hasEatenMap[key];            
        }
        else
        {
            if(ServerSettings.DebugEating) trace('${this.name} IncreaseFoodValue: craving hasEaten: NO!!!: key: $key, heldObject.id(): ${eatenFoodId}');
        }

        // restore also some biome loved food like bana for brown
        var lovedFoodId = getLovedFoodId();
        restoreFoodCount(lovedFoodId, amountEaten * ServerSettings.LovedFoodRestore);
            
        newHasEatenCount--;  // A food with full YUM is displayed as +1 craving 
        cravingHasEatenCount--; // A food with full YUM is displayed as +1 craving

        //if(newHasEatenCount >= 0) cravings.remove(eatenFoodId);
        //if(cravingHasEatenCount >= 0) cravings.remove(currentlyCraving);

        if(cravingHasEatenCount < 0 && currentlyCraving != 0 && currentlyCraving == eatenFoodId)
        {            
            if(ServerSettings.DebugEating) trace('${this.name} IncreaseFoodValue: craving: currentlyCraving: $currentlyCraving ${-cravingHasEatenCount}');

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
                if(ServerSettings.DebugEating) trace('${this.name} IncreaseFoodValue: no new craving / choose random new: Eaten: ${eatenFoodId}');

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
                    trace('${this.name} WARNING: No new random craving found!!!');
                    this.connection.send(ClientTag.CRAVING, ['${currentlyCraving} 0']); 
                    return;
                }

                var newObjData = ObjectData.foodObjects[index];

                if(hasEatenMap.exists(newObjData.id) == false) hasEatenMap[newObjData.id] = -1; // make sure to add it to the cravins and give a little boni

                newHasEatenCount = hasEatenMap[newObjData.id];
                newHasEatenCount--;

                if(ServerSettings.DebugEating) trace('${this.name} New random craving: ${newObjData.description} ${newObjData.id} lastCravingIndex: $lastCravingIndex index: $index  newHasEatenCount: ${-newHasEatenCount}');

                lastCravingIndex = index;
                currentlyCraving = newObjData.id;

                this.connection.send(ClientTag.CRAVING, ['${currentlyCraving} ${-newHasEatenCount}']); 
            }
            else
            {
                // look if there is a close food that can be craved
                currentlyCraving = 0; // ignore craving when search best food
                var bestfood = AiHelper.SearchBestFood(this, false);
                var newHasEatenCount = 1.0;
                var foodId = 0;
                
                if(bestfood != null)
                {
                    foodId = bestfood.objectData.getFoodId();
                    newHasEatenCount = hasEatenMap[foodId]; 
                    
                    if(ServerSettings.DebugEating) trace('${this.name} Find close craving: ${bestfood.name} newHasEatenCount: $newHasEatenCount');
                }

                if(newHasEatenCount > 0)
                {
                    // chose craving from known craving list
                    var random = WorldMap.calculateRandomInt(cravings.length -1);
                    foodId = cravings[random];
                    newHasEatenCount = hasEatenMap[foodId];
                }
                else displayFood(bestfood);

                newHasEatenCount--;
                this.connection.send(ClientTag.CRAVING, ['$foodId ${-newHasEatenCount}']);
                currentlyCraving = key;

                if(ServerSettings.DebugEating) trace('${this.name} IncreaseFoodValue: new craving: cravingHasEatenCount: $cravingHasEatenCount currentlyCraving: $currentlyCraving ${-newHasEatenCount}');
            }
        }            
    }

    private function restoreFoodCount(foodId:Int, amountEaten:Float)
    {
        this.hasEatenMap[foodId] -= amountEaten;
        var newHasEatenCount = hasEatenMap[foodId];
        var objData = ObjectData.getObjectData(foodId);

        if(ServerSettings.DebugEating) trace('${this.name} IncreaseFoodValue: amountEaten: $amountEaten food: ${objData.description} ==> ${Math.round(newHasEatenCount * 10) / 10}');

        if(newHasEatenCount <= 0 && cravings.contains(foodId) == false)
        {
            if(ServerSettings.DebugEating) trace('${this.name} IncreaseFoodValue: added craving: ${objData.description}');
            cravings.push(foodId);
        }
    }

    private static function doSwitchCloths(playerFrom:GlobalPlayerInstance, playerTo:GlobalPlayerInstance, clothingSlot:Int) : Bool
    {
        if(playerFrom.o_id[0] < 0) return false; // is holding player

        var objClothingSlot = playerFrom.calculateClothingSlot();

        if(objClothingSlot < 0 && playerFrom.heldObject.id != 0) return false;

        if(ServerSettings.DebugPlayer) trace('self:o_id: ${playerFrom.o_id[0]} helobj.id: ${playerFrom.heldObject.id} clothingSlot: $clothingSlot objClothingSlot: $objClothingSlot');

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

        if(this.o_id[0] < 1) return -1;
        
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

        if(ServerSettings.DebugPlayer) trace('objectData.clothing: ${objectData.clothing} objClothingSlot:  ${objClothingSlot}');
        //trace('clothingSlot:  ${clothingSlot}');

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

        AllPlayerMutex.acquire();
        Macro.exception(specialRemoveHelper(container, clothingSlot, index));
        Connection.SendUpdateToAllClosePlayers(this);

        AllPlayerMutex.release();
        return true;
    }

    private function specialRemoveHelper(container:ObjectHelper, clothingSlot:Int,index:Null<Int>)
    {
        if(this.o_id[0] < 0) return; // is holding player

        this.setHeldObject(container.removeContainedObject(index));

        setInClothingSet(clothingSlot);

        SetTransitionData(x,y, true);

        trace('this.clothing_set: ${this.clothing_set}');
    }

    public function isMeh(food:ObjectHelper) : Bool
    {
        return isYum(food) == false;
    }

    public function isYum(food:ObjectHelper) : Bool
    {
        var foodObjData = food.objectData;
        if(foodObjData.dummyParent != null) foodObjData = foodObjData.dummyParent;

        if(foodObjData.foodValue < 1) return false;

        var countEaten = hasEatenMap[foodObjData.id];

        return countEaten < ServerSettings.YumBonus; 
    }

    public function isHoldingYum() : Bool
    {
        if(this.o_id[0] < 0) return false; // is holding player

        return isYum(heldObject);
    }

    public function isHoldingMeh() : Bool        
    {
        if(this.o_id[0] < 0) return false; // is holding player

        var heldObjData = heldObject.objectData;
        if(heldObjData.dummyParent != null) heldObjData = heldObjData.dummyParent;

        if(heldObjData.foodValue < 1) return false;

        var countEaten = hasEatenMap[heldObjData.id];

        return countEaten > ServerSettings.YumBonus; 
    }
    
    public function isHoldingWeapon() : Bool
    {
        return heldObject.objectData.deadlyDistance > 0;
    }

    public function isHoldingMeleeWeapon() : Bool
    {
        return heldObject.objectData.deadlyDistance > 0 && heldObject.objectData.deadlyDistance < 2;
    }

    public function isHoldingRangedWeapon() : Bool
    {
        return heldObject.objectData.deadlyDistance >= 2;
    }

    public function setHeldObject(obj:ObjectHelper)
    {
        if(obj == null) obj = ObjectHelper.readObjectHelper(this, [0]);
        var player = this;
        this.heldObject = obj;

        // check if it is a light wound
        if(obj.isWound() && player.hiddenWound == null)
        {
            var transition = TransitionImporter.GetTransition(-1, obj.parentId, false, false);
            
            if(transition != null)
            {
                var alternativeTimeOutcome = obj.objectData.alternativeTimeOutcome; 
                var newid = alternativeTimeOutcome >=0 ? alternativeTimeOutcome : transition.newTargetID;

                var healthFactor = CalculateHealthFactor(2, 0.5);
                obj.timeToChange = ObjectHelper.CalculateTimeToChangeForObj(obj) / healthFactor;

                if(newid == 0)
                {
                    trace('is light wound: ${obj.name}');
                    player.hiddenWound = obj;
                }
            }
        }
        else if(obj != hiddenWound) obj.timeToChange = ObjectHelper.CalculateTimeToChangeForObj(obj); // TODO not ideal to set it here

        //trace('TIME22: SET ${obj.description} timeToChange: ${obj.timeToChange}');

        MakeSureHoldObjIdAndDummyIsSetRightAndNullObjUsed();

        if(player.heldObject.containedObjects.length > player.heldObject.objectData.numSlots)
        {
            var message = 'WARNING: setHeldObject: ${player.name} ${player.heldObject.name} ${player.heldObject.toArray()} slots: containedObjects.length > player.heldObject.objectData.numSlots: ${player.heldObject.objectData.numSlots}';
            trace(message);
            throw new Exception(message);
        }

        if(obj != null && obj.objectData.foodValue > 0)
        {
            // this was set so that display food displays text above player. 
            // but in an swap the position is not yet changed but it is tested in getObjectHelper if the position is correct which it is not yet...
            // in search best food if the position is still valid 
            //obj.tx = this.tx; 
            //obj.ty = this.ty;

            var isHoldingYum = isHoldingYum();
            if(isHoldingYum) this.doEmote(Emote.joy);
            else if(isSuperMeh(this.heldObject)) this.doEmote(Emote.sad);      
            else this.doEmote(Emote.hmph);
            
            if(ServerSettings.DisplayYumAndMehFood) displayFood(obj);
            DisplayBestFood(player);            
        }
    }

    public function isSuperMeh(food:ObjectHelper) : Bool
    {
        var foodValue:Float = food.objectData.foodValue;
        var countEaten = this.hasEatenMap[food.id]; 

        if(countEaten < 0) countEaten = 0;    
        foodValue += ServerSettings.YumBonus;
        foodValue -= countEaten;

        var isSuperMeh = foodValue < food.objectData.foodValue / 2;    
        return isSuperMeh;
    }

    public function displayFood(food:ObjectHelper)
    {
        if(this.isAi()) return;

        var player = this;
        var foodId = food.objectData.getFoodId();
        var countEaten = player.hasEatenMap[foodId];
        var isYum = countEaten < ServerSettings.YumBonus; 
        var text = 'F';
        var count;

        if(isYum)
        {
            text = foodId == currentlyCraving ? 'YY' : 'Y';            
            count = Math.ceil(ServerSettings.YumBonus - countEaten);
            count = Math.ceil(Math.min(5, count));
            for(i in 0...count) text += 'U';
            text += 'M!';
        }
        else
        {
            //var devide = Math.max(1, food.objectData.foodValue / 6);
            text = 'M';            
            count = Math.floor(1 + (countEaten - ServerSettings.YumBonus) / (food.objectData.foodValue / 4)); // three EEE if food has halve food value
            for(i in 0...count) text += 'E';
            text += 'H!';
        }

        //trace('DisplayBestFood: ${food.description} $text count: $count countEaten: $countEaten');

        player.connection.send(ClientTag.LOCATION_SAYS, ['${food.tx - player.gx} ${food.ty - player.gy} $text']);
    }

    public static function DisplayBestFood(player:GlobalPlayerInstance)
    {
        if(player.isAi()) return;
        //if(player.heldObject.objectData.foodValue <= 0) return;
        if(player.food_store > player.food_store_max * 0.8) return;
        if(player.currentlyCraving != 0 && player.heldObject.id == player.currentlyCraving) return;

        var bestfood = AiHelper.SearchBestFood(player);
        var displayBestFood = bestfood != null && (player.isHoldingYum() == false || bestfood.objectData.getFoodId() == player.currentlyCraving) && AiHelper.CalculateDistanceToObject(player, bestfood) > 10;

        if(displayBestFood) player.displayFood(bestfood);
    }

    public function getAi() : Ai
    {
        return this.connection.serverAi;
    }

    public function MakeSureHoldObjIdAndDummyIsSetRightAndNullObjUsed()
    {
        //if(this.o_id[0] < 0) return; // do nothing if a player is hold

        var obj = this.heldObject;

        if(obj == null)
        {
            obj = ObjectHelper.readObjectHelper(this, [0]);
            this.heldObject = obj;
        } 

        obj.TransformToDummy();
        this.o_id = this.heldPlayer == null ? obj.toArray() : [-this.heldPlayer.p_id];
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
            if(ServerSettings.DebugTransitionHelper) trace('transformHeldObject: ${fromObjData.id} --> ${toObjData.id} / numberOfUses set to 1');    
        }

        if(ServerSettings.DebugTransitionHelper) trace('transformHeldObject: heldObject.numberOfUses: ${heldObject.numberOfUses}');

        heldObject.id = id;
        setHeldObject(heldObject);
    }

    /**
        reason_disconnected
        reason_killed_id   (where id is the object that killed the player)
        reason_hunger
        reason_nursing_hunger  (starved while nursing a hungry baby)
        reason_age
    **/
    public function doDeath(deathReason:String)
    {
        AllPlayerMutex.acquire();

        Macro.exception(doDeathHelper(deathReason));

        AllPlayerMutex.release();
    }

    public function doDeathHelper(deathReason:String)
    {
        if(this.deleted) return;

        this.deleted = true;

        this.lineage.deathTime = TimeHelper.tick;
        this.lineage.age = this.age;
        this.lineage.trueAge = this.trueAge;
        this.lineage.deathReason = deathReason;
        this.lineage.prestige = this.prestige;
        this.lineage.coins = this.coins;

        var oldScore = this.account.totalScore;
        PlayerAccount.ChangeScore(this);  

        trace('Do death: ${name} score: ${oldScore} --> ${this.account.totalScore} ${deathReason} age: ${Math.floor(this.age)} trueAge: ${Math.floor(this.trueAge)} prestige: ${Math.floor(this.prestige)} coins: ${Math.floor(this.coins)}');

        this.age = this.trueAge; // bad health and starving can influence health, so setback true time a player lifed so that he sees in death screen
        this.reason = deathReason;
        
        ChooseNewLeader(this);

        // TODO set coordinates player based
        ServerSettings.startingGx = this.tx;
        ServerSettings.startingGy = this.ty;

        //this.connection.die();
        
        if(this.heldPlayer != null) this.dropPlayer(this.x, this.y); // TODO test
        if(this.heldByPlayer != null)
        {
            var player = this.heldByPlayer;
            var heldPlayer = this;

            heldPlayer.x = player.tx - heldPlayer.gx;
            heldPlayer.y = player.ty - heldPlayer.gy;
    
            player.heldPlayer = null;
            player.setHeldObject(null);
    
            heldPlayer.heldByPlayer = null;

            // TODO place baby bones in arms
        }

        placeGrave();
        InheritOwnership(this);
        InheritCoins(this);
        ScoreEntry.CreateScoreEntryForDeadRelative(this);

        Connection.SendUpdateToAllClosePlayers(this, false);       

        this.delete();   
    }

    private static function InheritCoins(player:GlobalPlayerInstance)
    {
        if(player.coins < 1) return;

        // TODO test
        // TODO only inherit if ally or family member is close by / otherwise place in grave for next visitor
        player.account.coinsInherited += player.coins * ServerSettings.InheritCoinsFactor;

        var bestPlayer = null;
        var score = 0.0;
        var coinsToInherit = player.coins;

        player.coins = 0;

        while(coinsToInherit >= 1)
        {
            for(p in AllPlayers)
            {             
                if(p.isAlly(player) == false && p.isSameFamily(player) == false) continue;

                var tmpScore = p.account.coinsInherited;

                if(p.isCloseRelative(player)) tmpScore *= 2;

                if(tmpScore < 1) continue;

                if(tmpScore > score)
                {
                    score = tmpScore;
                    bestPlayer = p;
                }
            }

            if(bestPlayer == null) break;

            var tmpCoins = Math.min(coinsToInherit, score);
            tmpCoins = Math.floor(tmpCoins);

            coinsToInherit -= tmpCoins;
            bestPlayer.coins += tmpCoins;
            bestPlayer.account.coinsInherited -= bestPlayer.isCloseRelative(player) ? tmpCoins / 2 : tmpCoins;
            bestPlayer.connection.sendGlobalMessage('You inherited $tmpCoins coins from ${player.name} because of your past actions!');

            trace('COINS: You inherited $tmpCoins coins from ${player.name} because of your past actions!');
        }

        // distribute coins to children // TODO what to do if no kids?
        if(coinsToInherit < 1) return;
        
        var children = player.getAllChildren(true);

        if(children.length < 1) return; // TODO store coins in grave

        var tmpCoins = coinsToInherit / children.length;

        for(c in children)
        {
            c.coins += tmpCoins;

            if(tmpCoins >= 1) bestPlayer.connection.sendGlobalMessage('You inherited ${Math.floor(tmpCoins)} coins from ${player.name}!');
            trace('COINS: You inherited ${Math.floor(tmpCoins)} coins from ${player.name}!');
        }    
    }

    private static function InheritOwnership(player:GlobalPlayerInstance)
    {
        for(obj in player.owning)
        {
            obj.removeOwner(player);
            
            if(player.followPlayer == null) continue;

            if(obj.hasOwners()) continue; // there are more people that own this

            obj.addOwner(player.followPlayer); // follow player should be the new sub leader if there is one

            // TODO pointer to property
            player.followPlayer.connection.sendGlobalMessage('You inherited a new property!'); 
        }

        // TODO what is if there is no new owner left
    }

    public static function ChooseNewLeader(deadLeader:GlobalPlayerInstance) : GlobalPlayerInstance
    {
        // TODO test
        var bestLeaderScore:Float = -1000;
        var bestLeader:GlobalPlayerInstance = null;
        var count = 0;

        for(p in AllPlayers) // Find best leader
        {
            if(p == deadLeader) continue;

            if(p.getTopLeader(deadLeader) != deadLeader) continue;

            count++;

            var score = p.account.totalScore;

            if(score < bestLeaderScore) continue;

            bestLeaderScore = score;
            bestLeader = p;
        }    

        if(bestLeader == null) return null;

        trace('New best leader: ${bestLeader.p_id} ${bestLeader.name} Score: $bestLeaderScore');

        // make new leader follow the leader the dead leader followed
        bestLeader.followPlayer = deadLeader.followPlayer;

        // Set new leader
        for(p in AllPlayers) 
        {
            if(p.followPlayer != deadLeader) continue;

            p.followPlayer = bestLeader;
        }

        // Let new leader exile same players
        for(p in AllPlayers) 
        {
            if(p.exiledByPlayers.exists(deadLeader.p_id) == false) continue;

            p.exiledByPlayers[bestLeader.p_id] = bestLeader;

            Connection.SendExileToAll(bestLeader, p);
        }

        // inform followers about new leader
        for(p in AllPlayers) 
        {
            if(p != bestLeader) continue;
            if(p.getTopLeader(bestLeader) != bestLeader) continue;            

            if(count >= 5)
            {
                p.connection.sendGlobalMessage('The old King ${deadLeader.name} died. Long live the new king ${bestLeader.name}!');
            }
            else
            {
                p.connection.sendGlobalMessage('The old leader ${deadLeader.name} died. Long live the new leader ${bestLeader.name}!');
            }
        }

        if(count >= 5)
        {
            bestLeader.connection.sendGlobalMessage('Your King ${deadLeader.name} died. You are the new King of $count people. Long live the King!');
        }
        else if(count > 0)
        {
            bestLeader.connection.sendGlobalMessage('Your leader ${deadLeader.name} died. You are the new leader of $count people. Be it worthy!');
        }

        deadLeader.followPlayer = bestLeader;

        return bestLeader;
    }

    public function getAllChildren(onlyLiving:Bool = true) : Array<GlobalPlayerInstance>
    {
        var children = new Array();

        for(c in AllPlayers)
        {
            if(onlyLiving && c.deleted) continue;
            if(c.mother == this || c.father == this) children.push(c);
        }

        return children;
    }

    public function placeGrave()
    {
        var grave:ObjectHelper;

        if(this.age < ServerSettings.MinAgeToEat)
        {
            grave = new ObjectHelper(this, 3053); // 3053 Baby Bone Pile
        } 
        else
        {
            grave = heldObject.isWound() ? new ObjectHelper(this, 752) : new ObjectHelper(this, 87); // 87 = Fresh Grave 88 = grave 752 = Murder Grave
        }

        if(this.heldObject != null && heldObject.isWound() == false) // dont place a Wound in grave
        {
            if(this.heldObject.isContainable())
            {
                grave.containedObjects.push(this.heldObject);
            }
            else
            {
                WorldMap.PlaceObject(this.tx, this.ty, this.heldObject, true); // TODO test for example with death on horse
            }

            this.setHeldObject(null);
        }

        // place the clothings in the grave, but not need to remove them from the player, since he is dead... //clothing_set:String = "0;0;0;0;0;0";
        for(obj in this.clothingObjects)
        {
            if(obj.id == 0) continue;

            grave.containedObjects.push(obj);
        }

        if(WorldMap.PlaceObject(this.tx, this.ty, grave, true) == false) trace('WARNING: could not place any grave for player: ${this.p_id}');

        Connection.SendGraveInfoToAll(grave);

        this.account.graves.push(grave);

        //ScoreEntry.CreateScoreEntryForCursedGrave(grave);
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

    // TODO increase with health
    public function calculateNotReducedFoodStoreMax() : Float
    {
        var p:GlobalPlayerInstance = this;
        
        var new_food_store_max:Float = ServerSettings.GrownUpFoodStoreMax;

        return new_food_store_max;
    }

    public function calculateFoodStoreMax() : Float
    {
        var p:GlobalPlayerInstance = this;
        var age = p.age;
        var healthFactor = CalculateSpeedMaxFoodStoreFactor();
        var new_food_store_max = calculateNotReducedFoodStoreMax() * healthFactor;

        if(age < 20) new_food_store_max = ServerSettings.NewBornFoodStoreMax + age / 20 * (ServerSettings.GrownUpFoodStoreMax - ServerSettings.NewBornFoodStoreMax);
        if(age > 50) new_food_store_max = ServerSettings.OldAgeFoodStoreMax + (60 - age) / 10 * (ServerSettings.GrownUpFoodStoreMax - ServerSettings.OldAgeFoodStoreMax);

        if(p.food_store < 0) new_food_store_max += ServerSettings.FoodStoreMaxReductionWhileStarvingToDeath * p.food_store;

        new_food_store_max -= p.hits;

        if(p.exhaustion > 0)
        {
            var tmp_food_store_max = new_food_store_max;

            new_food_store_max -= p.exhaustion;
        
            if(new_food_store_max < tmp_food_store_max / 2) new_food_store_max = tmp_food_store_max / 2;
        }

        return new_food_store_max;
    }
    /**
        KILL is for using a deadly object on the target square.  Square can
        be non-adjacent depending on deadly distance of held object.
        If another player is located there (even if moving and crossing)
        they will be killed.
        NOTE the alternate call for KILL with extra id parameter.
        this specifies a specific person to kill, if more than one is
        close to the target tile.
    **/
    public function kill(x:Int, y:Int, playerId:Int) : Bool // playerId = -1 if no specific player is slected
    {
        var result = false;

        AllPlayerMutex.acquire();

        Macro.exception(result = killHelper(x, y, playerId));

        AllPlayerMutex.release();

        return result;
    }

    public function killHelper(x:Int, y:Int, playerId:Int) : Bool // playerId = -1 if no specific player is slected
    {
        // TODO stop movement if hit
        // TODO block movement if not ally (with weapon?)     
        
        var targetPlayer = getPlayerAt(this.tx + x, this.tx + y, playerId);
        var name = targetPlayer == null ? 'not found!' : ${targetPlayer.name};
        var deadlyDistance = this.heldObject.objectData.deadlyDistance;
        
        if(targetPlayer == null)
        {
            this.connection.send(PLAYER_UPDATE, [this.toData()]);

            trace('kill: playerId: $playerId was not found!');

            return false;
        }

        if(targetPlayer.deleted)
        {
            this.connection.send(PLAYER_UPDATE, [this.toData()]);

            trace('kill: playerId: $playerId is allready dead!');

            return false;
        }

        if(ServerSettings.DebugCombat) trace('kill($x,$y ${targetPlayer.tx - this.gx},${targetPlayer.ty - this.gy} playerId: $playerId) ${name}');

        this.killMode = true;

        Connection.SendEmoteToAll(targetPlayer, Emote.shock);

        this.exhaustion += ServerSettings.CombatExhaustionCostPerAttack;
        targetPlayer.lastPlayerAttackedMe = this;

        // if player is not angry and none is in kill mode make angry first before attack is possible
        //
        //if(targetPlayer.angryTime > 0 && targetPlayer.killMode == false)
        if(this.angryTime > 0 || targetPlayer.angryTime > 0)
        {
            this.connection.send(PLAYER_UPDATE, [this.toData()]);

            var tmpAngry = Math.max(this.angryTime, targetPlayer.angryTime);

            tmpAngry = Math.ceil(tmpAngry);

            this.say('${tmpAngry} more seconds...');

            //trace('kill: needs to be $angryTime seconds more angry!');

            return false;
        }   
        
        // can only shoot at target with bow if not too close
        if(deadlyDistance > 1.9 && isCloseToPlayerUseExact(targetPlayer, 1.5))
        {
            this.connection.send(PLAYER_UPDATE, [this.toData()]);

            this.say('Too close...');

            //trace('kill: playerId: $playerId is allready dead!');

            return false;
        }

        Connection.SendEmoteToAll(this, Emote.murderFace);

        if(isCloseToPlayerUseExact(targetPlayer, deadlyDistance) == false)
        {
            this.connection.send(PLAYER_UPDATE, [this.toData()]);

            trace('kill: playerId: $playerId is too far away! deadlyDistance: $deadlyDistance');

            return false;
        }

        var quadDistance = AiHelper.CalculateDistance(x, y, targetPlayer.tx - gx, targetPlayer.ty - gy);
        var distanceFactor = 2 / (quadDistance + 2);

        if(distanceFactor < 0.3)
        {
            this.connection.send(PLAYER_UPDATE, [this.toData()]);

            trace('kill: playerId: $playerId is in range but target x,y is too far away! quadDistance: $quadDistance');

            return false;
        }

        if(targetPlayer.isAlly(this))
        {
            if(lastAttackedPlayer != targetPlayer)
            {
                this.connection.send(PLAYER_UPDATE, [this.toData()]);
                this.connection.sendGlobalMessage('${targetPlayer.name} is your ally! Attack again to exile!');

                lastAttackedPlayer = targetPlayer;

                trace('kill: playerId: $playerId is an ally!');

                return false;
            }
            else
            {
                //if(targetPlayer.getTopLeader() == this) this.exile(targetPlayer);
                this.exile(targetPlayer, false);
            }
        }

        targetPlayer.angryTime = -ServerSettings.CombatAngryTimeBeforeAttack; // make hit player angry, so that he can attack back
  
        var damage = targetPlayer.doDamage(this.heldObject, this, distanceFactor, quadDistance);
            
        /*this.setHeldObjectOriginNotValid(); // no animation
        targetPlayer.setHeldObjectOriginNotValid(); // no animation

        //this.connection.send(PLAYER_UPDATE, [this.toData()]);
        Connection.SendUpdateToAllClosePlayers(this);
        Connection.SendUpdateToAllClosePlayers(targetPlayer);
        Connection.SendDyingToAll(targetPlayer);*/

        var prestigeCost:Float = 0;

        if(targetPlayer.isHoldingWeapon() == false)
        {
            // TODO count as ally if exile happened not long ago ???
            // TODO auto exile if seen by leader ???
            if(targetPlayer.trueAge < ServerSettings.MinAgeToEat)
            {
                prestigeCost = damage * ServerSettings.PrestigeCostPerDamageForChild;

                prestigeCost = Math.ceil(prestigeCost);

                this.addHealthAndPrestige(-prestigeCost, false);

                this.connection.sendGlobalMessage('Lost $prestigeCost prestige for attacking a child ${targetPlayer.name}!');                 
            }
            else if(targetPlayer.isAlly(this))
            {
                prestigeCost = damage * ServerSettings.PrestigeCostPerDamageForAlly;

                prestigeCost = Math.ceil(prestigeCost);

                this.addHealthAndPrestige(-prestigeCost, false);

                this.connection.sendGlobalMessage('Lost $prestigeCost prestige for attacking ally ${targetPlayer.name}!');                 
            }
            else if(isCloseRelative(targetPlayer))
            {
                prestigeCost = damage * ServerSettings.PrestigeCostPerDamageForCloseRelatives;

                prestigeCost = Math.ceil(prestigeCost);

                this.addHealthAndPrestige(-prestigeCost, false);

                this.connection.sendGlobalMessage('Lost $prestigeCost prestige for attacking close relative ${targetPlayer.name}!');
            }
            else if(targetPlayer.isFemale())
            {
                prestigeCost = damage * ServerSettings.PrestigeCostPerDamageForWomenWithoutWeapon;

                prestigeCost = Math.ceil(prestigeCost);

                this.addHealthAndPrestige(-prestigeCost, false);

                this.connection.sendGlobalMessage('Lost $prestigeCost prestige for attacking a women without weapon ${targetPlayer.name}!');
            }
        }
        
        //trace('Wound: damage: $damage prestigeCost: $prestigeCost');

        return true;
    }

    public function doDamage(fromObj:ObjectHelper, attacker:GlobalPlayerInstance = null, distanceFactor:Float = 1, quadDistance:Float = 0) : Float
    {
        return DoDamage(this, fromObj, attacker, distanceFactor, quadDistance);
    }

    public function DoDamage(targetPlayer:GlobalPlayerInstance, fromObj:ObjectHelper, attacker:GlobalPlayerInstance = null, distanceFactor:Float = 1, quadDistance:Float = 0) : Float
    {
        // TODO do damage according to right / wrong biome

        var orgDamage = fromObj.objectData.damage * ServerSettings.WeaponDamageFactor;
        var damage = (orgDamage / 2) + (orgDamage * WorldMap.calculateRandomFloat());

        var protection = targetPlayer.calculateClothingInsulation();
        var protectionFactor = 1 / (protection + 1); // from 1 to 1 / 3;
        if(ServerSettings.DebugCombat) trace('COMBAT: protection: $protection protectionFactor: $protectionFactor');

        var allyFactor = 1.0;

        if(attacker != null)
        {
            if(targetPlayer.isAlly(attacker)) allyFactor = 0.5;
            else
            {
                targetPlayer.makeAllCloseAllyAngryAt(attacker);
                allyFactor = attacker.calculateEnemyVsAllyStrengthFactor();
                allyFactor = allyFactor > 1.2 ? 1.2 : allyFactor;
            }

            var isRightClassForWeapon = attacker.isRightClassForWeapon();
            if(ServerSettings.DebugCombat) trace('COMBAT: isRightClassForWeapon: $isRightClassForWeapon');

            damage *= attacker.isMale() ? ServerSettings.MaleDamageFactor : 1;
            damage *= allyFactor;
            damage *= distanceFactor;    
            damage *= isRightClassForWeapon ? 1.2 : 1; 
            damage *= attacker.isCursed ? ServerSettings.CursedDamageFactor : 1;
            damage *= attacker.isEveOrAdam() ? ServerSettings.EveDamageFactor : 1;

            if(ServerSettings.DebugCombat) trace('COMBAT: HIT weaponDamage: $orgDamage damage: $damage allyFactor: $allyFactor distanceFactor: $distanceFactor quadDistance: $quadDistance');
        }

        var isRightClassForWeapon = targetPlayer.isRightClassForWeapon();

        var doesRealDamage = fromObj.id != 2156; // 2156 Mosquito Swarm;
        var lovesJungle = targetPlayer.biomeLoveFactor(BiomeTag.JUNGLE);
        if(lovesJungle < -0.5) lovesJungle = -0.5;
        var moskitoDamageFactor = 1 / (1 + lovesJungle + yellowfeverCount); // without yellowfeverCount between 0.33 and 2
        var healthFactor = CalculateHealthFactor(2, 0.5);

        moskitoDamageFactor /= healthFactor;

        if(doesRealDamage == false) yellowfeverCount += 0.02;
        
        if(doesRealDamage) damage *= targetPlayer.heldObject.objectData.damageProtectionFactor;
        else damage *= moskitoDamageFactor;
        damage *= isRightClassForWeapon ? 0.8 : 1;
        damage *= targetPlayer.isEveOrAdam() ? ServerSettings.EveDamageFactor : 1;
        damage *= protectionFactor;
        damage *= targetPlayer.isWounded() ? ServerSettings.TargetWoundedDamageFactor : 1;

        targetPlayer.hits += damage;
        targetPlayer.exhaustion += damage;
        targetPlayer.food_store_max = targetPlayer.calculateFoodStoreMax();

        targetPlayer.sendFoodUpdate(false);
        if(doesRealDamage) Connection.SendDyingToAll(targetPlayer); // he is not actually dying but wounded

        if(ServerSettings.DebugCombat) trace('COMBAT: HIT objDamage: $orgDamage damage: $damage moskitoDamageFactor: $moskitoDamageFactor');

        if(targetPlayer.woundedBy == 0 || doesRealDamage) targetPlayer.woundedBy = fromObj.id;
        var longWeaponCoolDown = false;
        
        if(targetPlayer.food_store_max < 0)
        {
            longWeaponCoolDown = true;

            if(attacker != null && targetPlayer.coins > 0)
            {
                var coins = Math.floor(targetPlayer.coins * 0.8);
                attacker.coins += coins;
                targetPlayer.coins = 0;

                if(coins > 0) attacker.connection.sendGlobalMessage('You gained ${coins} from ${targetPlayer.name}!');
            } 

            targetPlayer.doDeath('reason_killed_${targetPlayer.woundedBy}');
        }

        var trans = TransitionImporter.GetTransition(fromObj.id, 0, true, false); // weapon
        if(trans == null) trans = TransitionImporter.GetTransition(fromObj.id, 0, false, false); // animal

        if(trans == null)
        {
            //trace('No Wound: ${fromObj.description}  ${fromObj.id}');
            
            //Connection.SendUpdateToAllClosePlayers(targetPlayer);
            return damage;
        }
        //trace('Wound: damage: ${damage} doesRealDamage: $doesRealDamage ' + trans.getDesciption());

        var doWound = targetPlayer.food_store_max < targetPlayer.calculateNotReducedFoodStoreMax() * fromObj.objectData.woundFactor;
        
        if(doesRealDamage == false) doWound = true; // TODO give a random chance
        if(doWound && targetPlayer.isWounded() == false) longWeaponCoolDown = true;

        if(doWound && targetPlayer.heldObject.isArrowWound() == false)
        {
            var newWound = new ObjectHelper(attacker, trans.newTargetID);

            if(doesRealDamage)
            {
                targetPlayer.killMode = false;

                if(targetPlayer.heldPlayer != null) dropPlayer(this.x, this.y); // TODO test
                
                if(targetPlayer.heldObject.id != 0)
                {
                    if(WorldMap.PlaceObject(targetPlayer.tx, targetPlayer.ty, targetPlayer.heldObject) == false) trace('WARNING: WOUND could not place heldobject player: ${targetPlayer.p_id}');
                }

                targetPlayer.setHeldObject(newWound);    
                targetPlayer.setHeldObjectOriginNotValid(); // no animation   
                Connection.SendUpdateToAllClosePlayers(targetPlayer); 
            }
            else
            {
                // in case of moskito check if target gets some yellow fever
                if(0.2 * Math.pow(moskitoDamageFactor,2) > WorldMap.calculateRandomFloat())
                {
                    yellowfeverCount += 1; // increases resistance
                    targetPlayer.fever = newWound;
                    
                    newWound.timeToChange = ObjectHelper.CalculateTimeToChangeForObj(newWound) * moskitoDamageFactor;
                    targetPlayer.connection.sendGlobalMessage('You got yellow fever from the moskitos. Try to cool down...');
                }
                targetPlayer.doEmote(Emote.sad);
                //targetPlayer.doEmote(Emote.yellowFever);
            }
            
            //trace('Wound2: ${targetPlayer.heldObject}');
        }
        else
        {
            // if it is an arrow wound, place arrow on ground if there is no wound
            var newWound = new ObjectHelper(this, trans.newTargetID);
            newWound.timeToChange = 2;
            WorldMap.PlaceObject(targetPlayer.tx, targetPlayer.ty, newWound, true);
            //trace('Wound3:');
        }  

        if(attacker == null) // attacker is animal
        {
            fromObj.id = trans.newActorID;
            //trace('Wound4: ${fromObj.id}');
        }
        else // attacker is player
        {
            fromObj.id = trans.newActorID;
            var bloodyWeapon = fromObj; //new ObjectHelper(attacker, trans.newActorID);
            attacker.setHeldObject(bloodyWeapon);
            attacker.heldObject.creationTimeInTicks = TimeHelper.tick;            
            
            attacker.setHeldObjectOriginNotValid(); // no animation
            Connection.SendUpdateToAllClosePlayers(attacker);
        }

        var timeTransition = TransitionImporter.GetTransition(-1, trans.newActorID);

        if(timeTransition != null)
        {
            var timeToChangeFactor = longWeaponCoolDown ? ServerSettings.WeaponCoolDownFactorIfWounding : ServerSettings.WeaponCoolDownFactor;
            // currently timetochange is overriden if object ist set
            //timeToChangeFactor = longWeaponCoolDown && attacker == null ? ServerSettings.AnimalCoolDownFactorIfWounding : timeToChangeFactor;

            fromObj.timeToChange = timeTransition.calculateTimeToChange() * timeToChangeFactor;
            if(ServerSettings.DebugCombat) trace('COMBAT: Bloody Weapon Time: ${fromObj.timeToChange} ' + timeTransition.getDesciption());
        }  

        // do damage to attacking animal
        // TODO bloody weapon
        if(attacker == null && targetPlayer.isHoldingMeleeWeapon())
        {
            fromObj.hits += isRightClassForWeapon ? 0.5 : 0.25;

            //trace('Damage: hits: ${fromObj.hits} ${fromObj.name}');

            if(fromObj.hits > WorldMap.calculateRandomInt(20))
            {
                //trace('Damage: dead: ${fromObj.hits} ${fromObj.name}');

                var tmpId = fromObj.id;
                
                if(fromObj.id == 427) fromObj.id = 422; // Wolf 418 --> Attacking Wolf 427--> Dead Wolf 422
                if(fromObj.id == 1333) fromObj.id = 1332; // Wild Boar 1323 --> Attacking Wild Boar 1333 --> Dead Boar 1332
                if(fromObj.id == 1328) fromObj.id = 1331; // Wild Boar with Piglet --> Shot Boar with Piglet

                if(fromObj.id != tmpId)
                {
                    WorldMap.world.setObjectHelper(fromObj.tx,fromObj.ty, fromObj);
                    Connection.SendMapUpdateToAllClosePlayers(fromObj.tx, fromObj.ty, [fromObj.id]);
                }
            }
        }
        
        //this.connection.send(PLAYER_UPDATE, [this.toData()]);

        return damage;
    }

    public function calculateEnemyVsAllyStrengthFactor() : Float
    {        
        var allyStrength = 0.0;
        var enemyStrength = 0.0;

        for(p in AllPlayers)
        {
            if(p.deleted) continue;
            if(p.isCloseToPlayer(this, ServerSettings.AllyConsideredClose) == false) continue;

            var strength = p.isHoldingWeapon() ? 2 * p.food_store_max : p.food_store_max;

            if(p.isFriendly(this)) allyStrength += strength;
            else enemyStrength += strength;
        }

        var factor = (allyStrength + allyStrength) / (enemyStrength + allyStrength);

        //if(ServerSettings.DebugCombat)
        trace('ALLY STRENGTH: ${allyStrength} vs enemy: ${enemyStrength} factor: $factor');

        return factor;
    }

    // TODO test // TODO better make angry in timehelper if enemy is close?
    public function makeAllCloseAllyAngryAt(angryAtplayer:GlobalPlayerInstance) 
    {        
        for(p in AllPlayers)
        {
            if(p.deleted) continue;
            if(p.isCloseToPlayer(this, ServerSettings.AllyConsideredClose) == false) continue;

            if(p.isAlly(this)) p.lastPlayerAttackedMe = angryAtplayer;
        }
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
        var targetPlayer = getPlayerAt(this.tx + x, this.tx + y, playerId);

        //trace('doBaby($x, $y playerId: $playerId)');

        AllPlayerMutex.acquire();
        
        var done = false;
        Macro.exception(done = doBabyHelper(x,y, targetPlayer));
        // send always PU so that player wont get stuck
        if(done == false)
        {
            this.connection.send(PLAYER_UPDATE,[this.toData()]);
            this.connection.send(FRAME);
        }

        AllPlayerMutex.release();

        return done;
    }

    public function doBabyHelper(x:Int, y:Int, targetPlayer:GlobalPlayerInstance) : Bool
    {
        if(targetPlayer == null)
        {
            trace('doBaby: could not find target player!');
            return false;
        }

        if(isCloseToPlayer(targetPlayer) == false)
        {
            trace('doBaby: x,y is too far away!');
            return false;
        }

        if(this.o_id[0] != 0)
        {
            trace('Cannot pickup player, since hands are not empty ${this.o_id[0]}!');
            return false;
        }

        if(targetPlayer.age >= ServerSettings.MaxAgeForAllowingClothAndPrickupFromOthers ) // TODO allow pickup of knocked out players 
        {
            trace('Cannot pickup player, player is too old! player.age: ${targetPlayer.age}');
            return false;
        }

        // since targetPlayer may have moved inform where the player is now
        Connection.SendUpdateToAllClosePlayers(targetPlayer, true);

        this.heldPlayer = targetPlayer;
        targetPlayer.heldByPlayer = this;
        this.setHeldObject(null);

        if(targetPlayer.heldObject.isDroppable())
        {
            WorldMap.PlaceObject(this.tx, this.ty, targetPlayer.heldObject, false);
            targetPlayer.setHeldObject(null);
        }

        this.exhaustion += ServerSettings.PickupExhaustionGain;

        if(this.isFertile() && targetPlayer.age < ServerSettings.MaxChildAgeForBreastFeeding)
        {
            if(targetPlayer.food_store < targetPlayer.getMaxChildFeeding())
            {
                var food = ServerSettings.PickupFeedingFoodRestore;
                this.food_store -= food / 2;
                targetPlayer.food_store += food;

                this.sendFoodUpdate();
                targetPlayer.sendFoodUpdate();
            }
        }

        this.SetTransitionData(x,y,true);

        //trace('doBabyHelper: o_id:  ${this.o_id}');

        Connection.SendUpdateToAllClosePlayers(this, true);

        var followPlayer = targetPlayer.followPlayer;

        if(followPlayer == null || (followPlayer.isFertile() == false && this.isFertile()))
        {
            targetPlayer.followPlayer = this; // consider this player as mother
        }

        return true;
    }

    public function dropPlayer(x:Int, y:Int) : Bool
    {
        //trace('drop player');

        AllPlayerMutex.acquire();
        
        if(this.heldPlayer == null)
        {
            this.connection.send(PLAYER_UPDATE,[this.toData()]);
            AllPlayerMutex.release();   
            return false;    
        }

        var done = false;
        Macro.exception(done = dropPlayerHelper(x, y));

        AllPlayerMutex.release();

        return done;
    }

    private function dropPlayerHelper(x:Int, y:Int) : Bool
    {
        //trace('drop player helper');
        var player = this;
        var heldPlayer = player.heldPlayer;

        if(player.isClose(x, y, 1) == false)
        {
            trace('Drop target $x,$y is too far away!');
            return false;
        }

        heldPlayer.x = x + player.gx - heldPlayer.gx;
        heldPlayer.y = y + player.gy - heldPlayer.gy;
        heldPlayer.moveHelper.exactTx = heldPlayer.tx;
        heldPlayer.moveHelper.exactTy = heldPlayer.ty;

        player.heldPlayer = null;
        player.o_id = [0];

        heldPlayer.heldByPlayer = null;
        heldPlayer.forced = true;
        heldPlayer.responsible_id = player.p_id;
        heldPlayer.done_moving_seqNum += 1;

        Connection.SendUpdateToAllClosePlayers(heldPlayer, true, false);
        Connection.SendUpdateToAllClosePlayers(player,true);

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

        AllPlayerMutex.acquire();

        if(this.heldByPlayer == null)
        {
            this.connection.send(PLAYER_UPDATE,[this.toData()]);
            this.connection.sendWiggle(this);
            this.connection.send(FRAME, null, true);

            AllPlayerMutex.release();

            return false;    
        }

        var done = false;

        //var done = doHelper(this.heldByPlayer, this, dropPlayerHelper);
        Macro.exception(done = this.heldByPlayer.dropPlayer(heldByPlayer.x, heldByPlayer.y));

        AllPlayerMutex.release();

        return done;
    }

    private static function doHelper(player:GlobalPlayerInstance, targetPlayer:GlobalPlayerInstance, doFunction:GlobalPlayerInstance->Bool) : Bool
    {
        var done = false;     

        Macro.exception(done = doFunction(player));

        // send always PU so that player wont get stuck
        if(done == false)
        {
            player.connection.send(PLAYER_UPDATE,[player.toData()]);
            player.connection.send(FRAME);
        }

        return done;
    }

    public function isFertile() : Bool
    {
        if(deleted) return false;
        if(this.age < ServerSettings.MinAgeFertile || this.age > ServerSettings.MaxAgeFertile) return false;

        return isFemale();
    }

    public function isFemale() : Bool
    {
        var person = ObjectData.getObjectData(this.po_id);
        return person.male == false; 
    }

    public function isMale() : Bool
    {
        var person = ObjectData.getObjectData(this.po_id);
        return person.male; 
    }

    private static function DoDebugCommands(player:GlobalPlayerInstance, text:String)
    {
        if(text.indexOf('!HIT H') != -1)
        {
            trace('!HIT HELD');

            if(player.heldPlayer == null) return;

            player.heldPlayer.hits += 3;
            player.heldPlayer.food_store_max = player.calculateFoodStoreMax();

            return;
        }

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

            player.setHeldObject(new ObjectHelper(null, 1363));
            player.heldObject.timeToChange *= 0.2;

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
            Server.server.map.generateExtraDebugStuff(player.tx, player.ty);
        }
        else if(text.indexOf('!CREATE') != -1) // "create xxx" with xxx = id
        {
            trace('Create debug object');

            var id = findObjectByCommand(text);

            if(id < 0) return;
            
            WorldMap.world.setObjectId(player.tx, player.ty, [id]);

            Connection.SendMapUpdateToAllClosePlayers(player.tx, player.ty, [id]);
        }
        else if(text.indexOf('!CLOSE') != -1) 
        {
            trace('Close connection');

            player.connection.close();
        }
        else if(text.indexOf('!FOOD') != -1) 
        {
            player.food_store -= 5;
            player.sendFoodUpdate(false);
        }
        else if(text.indexOf('!YUM') != -1) 
        {
            player.food_store += 5;
            player.sendFoodUpdate(false);
        }
        else if(text.indexOf('!AGE') != -1) 
        {
            player.age += 5;
            player.trueAge += 5;
            Connection.SendUpdateToAllClosePlayers(player);
            //player.sendFoodUpdate(false);
        }
        else if(text.indexOf('!JAI') != -1) 
        {
            var ais = Connection.getLivingAis();

            if(ais.length > 0)
            {
                var ai = ais[WorldMap.calculateRandomInt(ais.length -1)];
                player.x = ai.connection.player.tx - player.gx;
                player.y = ai.connection.player.ty - player.gy;

                player.forced = true;
                Connection.SendUpdateToAllClosePlayers(player);
                player.forced = false;
                
                player.connection.sendMapChunk(player.x,player.y);
            }
        }
        else if(text.indexOf('!JHUMAN') != -1) 
        {
            var livingHumans = Connection.getLivingHumans();

            if(livingHumans.length > 0)
            {
                var p = livingHumans[WorldMap.calculateRandomInt(livingHumans.length -1)];
                player.x = p.tx - player.gx;
                player.y = p.ty - player.gy;

                player.forced = true;
                Connection.SendUpdateToAllClosePlayers(player);
                player.forced = false;

                player.connection.sendMapChunk(player.x,player.y);
            }
        }
    }

    
    public static function findObjectByCommand(text:String) : Int
    {
        var strings = text.split(' ');

        if(strings.length < 2) return -1;

        var id = Std.parseInt(strings[1]);

        //trace('${strings[1]} $id');

        var toSearch = StringTools.replace(text, '${strings[0]} ', '');        
        var end = toSearch.contains('!');
        toSearch = StringTools.replace(toSearch, '!', '');

        trace('Command Search: /${toSearch}/ end: $end');

        if(id != null) return id;

        id = ObjectData.GetObjectByName(toSearch, false, end);

        return id;
    }

    public function isAi() : Bool
    {
        //return this.connection.playerAccount.isAi;
        return this.connection.isAi();   
    }

    public function isHuman() : Bool
    {
        return this.isAi() == false;
        //return this.connection.serverAi != null;   
    }

    public function isHoldingChildInBreastFeedingAgeAndCanFeed() : Bool
    {
        if(heldPlayer == null) return false;
        if(heldPlayer.age > ServerSettings.MaxChildAgeForBreastFeeding) return false;
        if(this.food_store < 0) return false;
        return this.isFertile();
    }

    public function isSuperHot()
    {
        var tooHot = 0.5 + 0.5 * ServerSettings.TemperatureImpactBelow;
        var color = this.getColor();

        if(color == PersonColor.Black) tooHot += 0.2;
        if(color == PersonColor.Brown) tooHot += 0.1;
        if(color == PersonColor.White) tooHot += 0.05;

        return (this.heat > tooHot);
    }

    public function isSuperCold()
    {
        var tooCold = 0.5 - 0.5 * ServerSettings.TemperatureImpactBelow;
        var color = this.getColor();

        if(color == PersonColor.Ginger) tooCold -= 0.2;
        if(color == PersonColor.White) tooCold -= 0.1;
        if(color == PersonColor.Brown) tooCold -= 0.05;

        return (this.heat < tooCold);
    }

    /** Displayes from -X to plus X if biome is loved with 0 equals a neutral biome.
    A (brown) child with both parents same color loves (jungle) with 2. 
    A (brown) child with none parents same color loves (jungle) with 1. 
    A child with different color then (both) brown parents loves (jungle) with 0.5
    A child with different color then (one) brown parent loves (jungle) with 0
    A child with different color and no brown parent hates (jungle) with -0.5
    River or Swamp is hated with -2.5 if not on a floor.
    **/
    public function biomeLoveFactor(biome:Int = -1) : Float
    {
        var world = WorldMap.world;
        var floor = world.getFloorId(this.tx, this.ty);
        
        var loved:Float = 0;
        if(biome < 0) biome = world.getBiomeId(this.tx, this.ty);

        loved += BiomeLoveFactorForColor(biome, this, floor);
        if(this.mother != null) loved += BiomeLoveFactorForColor(biome, this.mother, floor, true);
        if(this.father != null) loved += BiomeLoveFactorForColor(biome, this.father, floor, true);

        return loved;
    }
    
    public static function BiomeLoveFactorForColor(biome:Int, player:GlobalPlayerInstance, floorId:Int, motherOrFather:Bool = false)
    {
        var loved:Float = 0;

        if(Biome.IsBiomeLovedbyColor(biome, player)) loved += 1;
        /*if(biome == BiomeTag.SNOW && personColor == PersonColor.Ginger) loved += 1;
        if(biome == BiomeTag.GREY && personColor == PersonColor.White) loved += 1;
        if(biome == BiomeTag.JUNGLE && personColor == PersonColor.Brown) loved += 1;
        if(biome == BiomeTag.DESERT && personColor == PersonColor.Black) loved += 1;*/

        if(motherOrFather == false && loved <= 0 && biome != BiomeTag.GREEN && biome != BiomeTag.YELLOW) loved -= 0.5;
        // only reduction if on bridge or floor in swamp or passableriver
        if(motherOrFather == false && loved <= 0 && floorId != 0 && (biome == BiomeTag.SWAMP || biome == BiomeTag.PASSABLERIVER)) loved -= 2.5;

        if(motherOrFather) loved *= 0.5;

        return loved;
    }

    public var lineagePrestige(get, null):Float;

    public function get_lineagePrestige()
    {
        return this.account.totalScore;
    }

    public var prestige(get, null):Float;

    public function get_prestige()
    {
        return this.yum_multiplier;
    }

    public function addPrestige(count:Float)
    {
        this.yum_multiplier += count;
    }

    public function addHealthAndPrestige(count:Float, isFood:Bool = true)
    {        
        this.yum_multiplier += count;
        if(isFood) this.prestigeFromEating += count;

        if(count <= 0) return;

        this.coins += count;

        var tmpCount = count; 

        if(this.mother != null)
        {
            mother.yum_multiplier += tmpCount / 2;
            mother.prestigeFromChildren += tmpCount / 2;

            if(this.mother.mother != null) // grandma
            {
                mother.mother.yum_multiplier += tmpCount / 4;
                mother.mother.prestigeFromChildren += tmpCount / 4;
            }

            if(this.mother.father != null) // grandpa
            {
                mother.father.yum_multiplier += tmpCount / 4;
                mother.father.prestigeFromChildren += tmpCount / 4;
            }
        }

        if(this.father != null)
        {
            father.yum_multiplier += tmpCount / 2;
            father.prestigeFromChildren += tmpCount / 2;

            if(this.father.mother != null) // grandma
            {
                father.mother.yum_multiplier += tmpCount / 4;
                father.mother.prestigeFromChildren += tmpCount / 4;
            }

            if(this.father.father != null) // grandpa
            {
                father.father.yum_multiplier += tmpCount / 4;
                father.father.prestigeFromChildren += tmpCount / 4;
            }
        }

        var children = this.getAllChildren();

        // prestige for children
        for(child in children)
        {
            child.yum_multiplier += tmpCount / 4;
            child.prestigeFromParents += tmpCount / 4;
        }
        
        // prestige for siblings
        if(mother != null && children.length > 0)
        {
            var sibling = children[WorldMap.calculateRandomInt(children.length -1)];
            sibling.yum_multiplier += tmpCount / 2;
            sibling.prestigeFromSiblings += tmpCount / 2;
        }            

        if(this.getTopLeader() == null) return;

        tmpCount = count / 5;

        var leader = followPlayer;
        if(leader == null) return;

        for(ii in 0...5)
        {
            if(this.exiledByPlayers.exists(leader.p_id)) return; // is exiled

            leader.yum_multiplier += tmpCount;
            leader.prestigeFromFollowers += tmpCount;
            leader.coins += tmpCount;

            if(leader.followPlayer == null) return;

            leader = leader.followPlayer;
        }
    }

    public function isWounded() : Bool
    {
        return this.heldObject.isWound();  
    } 

    public function isAlly(target:GlobalPlayerInstance) : Bool
    {
        if(target == null) return false;
        return this.getTopLeader() == target.getTopLeader();
    }

    public function isSameFamily(target:GlobalPlayerInstance) : Bool
    {
        return this.lineage.myEveId == target.lineage.myEveId;
    }

    public function isCloseRelative(target:GlobalPlayerInstance) : Bool
    {
        if(target == this.mother) return true;  // target is mother  
        if(target == this.father) return true;  // target is father

        if(target.mother == this) return true;  // target is kid   
        if(target.father == this) return true;  // target is kid
        
        if(target.mother == this.mother) return true; // brother / sister   
        if(target.father == this.father) return true; // brother / sister 
        
        if(target.mother != null && target.mother.mother == this) return true;  // target is grandkid
        if(target.father != null && target.father.father == this) return true;  // target is grandkid

        if(this.mother != null && this.mother.mother == this) return true;  // target is grandma
        if(this.father != null && this.father.father == this) return true;  // target is grandpa

        return false;
    }

    public function isMyGrave(obj:ObjectHelper) : Bool
    {
        for(grave in account.graves)    
        {
            if(grave == obj) return true;
        }

        return false;
    }

    public override function isHeld() : Bool
    {
        return (this.heldByPlayer != null);
    }

    public var account(get, null):PlayerAccount;

    public function get_account()
    {
        return this.connection.playerAccount;
    }

    public function getClosePlayer(maxDistance:Float = 1.5, hostile:Bool = true, hasWeapon = true) : GlobalPlayerInstance
    {
        for(p in AllPlayers)
        {
            if(p.deleted) continue;
            if(p.isCloseToPlayerUseExact(this, maxDistance) == false) continue;
            if(hostile && p.isFriendly(this)) continue;
            if(hasWeapon && p.isHoldingWeapon() == false) continue;

            return p;
        }

        return null;
    }

    public function isFriendly(player:GlobalPlayerInstance) : Bool
    {
        return this.isAlly(player) && this.lastAttackedPlayer != player && this.lastPlayerAttackedMe != player;
    }

    public function isHostile(player:GlobalPlayerInstance) : Bool
    {
        return isFriendly(player) == false;
    }

    public function isBlocked(tx:Int, ty:Int) : Bool 
    {
        var player:GlobalPlayerInstance = this;
        var obj = WorldMap.world.getObjectHelper(tx, ty);

        if(obj.blocksWalking()) return true;
        
        if(player.heldObject.objectData.isBoat)
        {
            var biome = WorldMap.world.getBiomeId(tx, ty);
            if(Biome.IsWater(biome)) return false;
        }
        
        return WorldMap.isBiomeBlocking(tx, ty); 
    }

    public function isDeleted()
    {
        return deleted;   
    }

    public function isEveOrAdam() : Bool
    {
        return (this.name == "EVE" || this.name == "ADAM");
    }

    public function getCraving() : Int
    {
        return this.currentlyCraving;    
    }

    public function getCountEaten(foodId:Int) : Float
    {
        return this.hasEatenMap[foodId];    
    }

    public function isRightClassForWeapon() : Bool
    {
        if(this.isHoldingWeapon() == false) return false;

        var player = this;
        var weaponPrestigeClass:Int = player.heldObject.objectData.prestigeClass;
        var attackerPrestigeClass:Int = player.lineage.prestigeClass;
        var isRightClassForWeapon = weaponPrestigeClass > 0 && weaponPrestigeClass <= attackerPrestigeClass;

        return isRightClassForWeapon;
    }

    public function hasYellowFever() : Bool
    {
        return fever != null && fever.id == 2155; // 2155 Yellow Fever
    }

    public function canFeedToMe(food:ObjectHelper) : Bool
    {
        if(isMeh(food) && food_store > 2) return false;
        if(food.id == 837) return false; // dont feed 837 ==> Psilocybe Mushroom to others

        return canEat(food);
    }

    public function canEat(food:ObjectHelper) : Bool
    {
        var objData = food.objectData;
        if(objData.dummyParent != null) objData = objData.dummyParent;    
        var originalFoodValue = objData.foodValue;

        if(originalFoodValue <= 0) return false;

        if(isSuperMeh(food) && food_store > 2) return false; 

        return this.food_store_max - this.food_store >= Math.ceil(originalFoodValue / 4);
    }

    public function getLovedFoodId() : Int
    {
        var lovedBiome = Biome.GetLovedBiomeByPlayer(this);
        return Biome.getLovedFoodId(lovedBiome);
    }

    public function getLovedPlants() : Array<Int>
    {
        var lovedBiome = Biome.GetLovedBiomeByPlayer(this);
        return Biome.getLovedPlants(lovedBiome);
    }

    public function getMaxChildFeeding() : Float
    {
        return Math.max(4, food_store_max);
    }

    public function isIll() : Bool
    {
        return fever != null;   
    }
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
    // by default you follow your mother
    // if your mother / father dies, you follow the noble of the mother / father
    // people in a village are distributed as followers among the nobles if a nobles dies
    
    // TODO Arcurus>> prince
    // if you have the highest score in this village (not counting the leader score) you are born as prince / princess to the leader
    // the eldest prince / princess becomes the crown prince
    // if there is no prince the noble with the highest score in this village becomes Cancelor
    // exiles / commands from crown prince / cancelor are valid for all followers if not overriden by the leader
    // giving a crown from the leader to a noble or prince makes them the new Cancelor / crown prince as long as he keeps the crown. 
    // A cancelor with a crown will get the new leader in case of the leaders death