package openlife.server;
import openlife.macros.Macro;
import openlife.server.GlobalPlayerInstance.Emote;
import openlife.data.object.ObjectHelper;
import openlife.data.object.ObjectData;
import openlife.settings.ServerSettings;
import openlife.data.Pos;

//@:multiReturn extern class NewMovements {
private class NewMovements
{
    public var moves:Array<Pos> = [];

    public var length:Float;

    // biome speed of starting Tile
    public var startSpeed:Float;

    // biome speed of last Movement Tile
    public var endSpeed:Float;

    // complete speed of last Movement Tile
    public var finalSpeed:Float;

    // true if movement was cut    
    public var trunc:Int;

    public var fullPathHasRoad = false;
    
    public function new() {}
}


class MoveHelper
{
    private static var firstPos = new Pos(); // used for calculation of movement length. Always stays 0,0

    public var player:GlobalPlayerInstance;
    public var waitForForce = false;

    // x,y when last chunk was send
    private var tx:Int = 0;
    private var ty:Int = 0;

    public var exactTx:Float = 0; // exact absolute x position calculated each time step
    public var exactTy:Float = 0; // exact absolute y position calculated each time step
    private var timeExactPositionChangedLast:Float = 0; // in absolute ticks

    // to calculate if the move is finished
    private var newMoveSeqNumber:Int = 0; 
    private var newMovements:NewMovements;
    private var newMoves:Array<Pos>;
    private var totalMoveTime:Float = 0;
    private var startingMoveTicks:Float = 0;

    public function new(player:GlobalPlayerInstance)
    {
        this.player = player;
    }

    public function isMoveing():Bool
    {    
        return (this.newMoves != null);
    }

    public function guessX()
    {
        return Math.round(exactTx) - player.gx;    
    }

    public function guessY()
    {
        return Math.round(exactTy) - player.gy;    
    }

    public function isCloseToPlayerUseExact(targetPlayer:GlobalPlayerInstance, distance:Float = 1) : Bool
    {    
        var target = targetPlayer.moveHelper;
        return isCloseUseExact(target.exactTx, target.exactTy, distance);
    }

    public function isCloseUseExact(targetTx:Float, targetTy:Float, distance:Float = 1) : Bool
    {    
        var xDiff = this.exactTx - targetTx;
        var yDiff = this.exactTy - targetTy;

        return (xDiff * xDiff + yDiff * yDiff <= distance * distance);
    }
    
    public function calculateExactQuadDistance(targetTx:Float, targetTy:Float) : Float
    {    
        var xDiff = this.exactTx - targetTx;
        var yDiff = this.exactTy - targetTy;

        return (xDiff * xDiff + yDiff * yDiff);
    }

    static public function calculateSpeed(p:GlobalPlayerInstance, tx:Int, ty:Int, fullPathHasRoad:Bool = true) : Float
    {
        var map =  Server.server.map;
        var onHorseOrCar = p.heldObject.objectData.speedMult >= 1.1;
        var speed = ServerSettings.InitialPlayerMoveSpeed;
        var floorObjData = ObjectData.getObjectData(map.getFloorId(tx,ty));
        var floorSpeed = floorObjData.speedMult;
        var onRoad = false;
        var hasBothShoes = p.hasBothShoes();       
        var isOnBoat = p.heldObject.objectData.isBoat; 

        if(ServerSettings.AutoFollowAi && p.isHuman()) return 2 * speed;
        if(ServerSettings.DebugSpeed) trace('speed: hasBothShoes: $hasBothShoes');
        if(hasBothShoes && onHorseOrCar == false) speed *= 1.1;
        if(fullPathHasRoad == false) floorSpeed = 1; // only consider road if the full path is on road

        onRoad = floorSpeed >= 1.01; // only give road speed boni if full path is on road

        speed *= ServerSettings.SpeedFactor; // used to increase speed if for example debuging
        speed *= floorSpeed;

        // DO biomes
        var biomeSpeed = map.getBiomeSpeed(tx,ty);  

        // road reduces speed mali of bad biome
        if((onRoad || isOnBoat) && biomeSpeed < 0.99) biomeSpeed = 1; //biomeSpeed = Math.sqrt(biomeSpeed);

        speed *= biomeSpeed;

        // DO speed held objects
        var speedModHeldObj = p.heldObject.objectData.speedMult;

        if(biomeSpeed < 0.999 && speedModHeldObj > 1) // horses and cars are bad in bad biome 
        {
            if(speedModHeldObj > 2.50) speedModHeldObj = 0.8; // super speedy stuff like cars
            else if(speedModHeldObj > 1.8) speedModHeldObj = 0.9; // for example horse
            else if(speedModHeldObj > 1.2) speedModHeldObj = 0.8; // for example horse cart
            
            if(ServerSettings.DebugSpeed) trace('Speed: New ${p.heldObject.objectData.description} speed in bad biome: ${p.heldObject.objectData.speedMult} --> $speedModHeldObj');
        }
        
        if(onRoad && speedModHeldObj < 0.99) speedModHeldObj = Math.sqrt(speedModHeldObj); // on road
        speed *= speedModHeldObj;

        // Do speed hidden wound
        if(p.hiddenWound != null && p.hiddenWound != p.heldObject) speed *= p.hiddenWound.objectData.speedMult;

        // make cars to boats
        if(isOnBoat && WorldMap.world.isWater(tx,ty) == false)
        {            
            speed = 0.5 * ServerSettings.InitialPlayerMoveSpeed;
        }

        // DO speed contained objects
        // TODO half penalty for strong 
        var containedObjSpeedMult:Float = 1;
        var backpack = p.getPackpack();

        for(obj in backpack.containedObjects)
        {
            containedObjSpeedMult *= calculateObjSpeedMult(obj);             
        }

        if(hasBothShoes) containedObjSpeedMult = Math.sqrt(containedObjSpeedMult);
        if(ServerSettings.DebugSpeed) trace('speed: backpack: containedObjSpeedMult: $containedObjSpeedMult');
        
        for(obj in p.heldObject.containedObjects)
        {
            containedObjSpeedMult *= calculateObjSpeedMult(obj); 
            
            for(subObj in obj.containedObjects)
            {
                containedObjSpeedMult *= calculateObjSpeedMult(subObj); 
            }
        }

        if(biomeSpeed < 0.9 && onRoad == false) containedObjSpeedMult *= containedObjSpeedMult; // in bad biome and off road double mali

        if(onRoad && containedObjSpeedMult < 0.99) containedObjSpeedMult = Math.sqrt(containedObjSpeedMult); // on road

        if(onHorseOrCar && containedObjSpeedMult < 0.99) containedObjSpeedMult = Math.sqrt(containedObjSpeedMult); // on horse / in car // TODO or strong

        if(containedObjSpeedMult < 1 && ServerSettings.DebugSpeed) trace('Speed: containedObjSpeedMult ${containedObjSpeedMult}');

        speed *= containedObjSpeedMult;

        // Reduce speed if damaged or age 
        var fullHitpoints = ServerSettings.GrownUpFoodStoreMax;
        var currenHitpoints = p.calculateFoodStoreMax();

        // between 1/2 and 1 if currenHitpoints <= fullHitpoints
        speed *= (currenHitpoints + fullHitpoints) / (fullHitpoints + fullHitpoints);

        if(ServerSettings.DebugSpeed) trace('SPEED: $speed currenHitpoints: $currenHitpoints fullHitpoints: $fullHitpoints');
        
        // Do temperature speed
        var temperatureSpeedImpact = ServerSettings.TemperatureSpeedImpact;
        if(p.isSuperHot())  speed *= p.heat > 0.98 ? Math.pow(temperatureSpeedImpact,2) : temperatureSpeedImpact;
        else if(p.isSuperCold()) speed *= p.heat < 0.02 ? Math.pow(temperatureSpeedImpact,2) : temperatureSpeedImpact;

        if(p.account.hasCloseBlockingGrave(p.tx, p.ty))
        {
            if(p.isCursed == false)
            {
                Connection.SendCurseToAll(p); // TODO test
                p.say('My grave is near...', true);
                p.doEmote(Emote.sad);
                p.connection.sendGlobalMessage('Since you are near your old bones you are cursed!');
            }

            speed *= ServerSettings.CloseGraveSpeedMali;
            p.isCursed = true;
        }
        else
        {
            if(p.isCursed == true)
            {
                Connection.SendCurseToAll(p, 0);
                p.say('Im far away from my grave...', true);
                p.doEmote(Emote.happy);
            }
            
            p.isCursed = false;
        }

        var biomeLoveFactor = p.biomeLoveFactor();
        if(biomeLoveFactor < 0 && p.inWrongBiome == false)
        {
            p.inWrongBiome = true;
            p.doEmote(Emote.homesick);
        }
        else if(biomeLoveFactor >= 0 && p.inWrongBiome)
        {
            p.inWrongBiome = false;
            //p.doEmote(Emote.biomeRelief);
        }
        else if(biomeLoveFactor > 0 && p.inHomeBiome == false)
        {
            p.inHomeBiome = true;
            p.doEmote(Emote.biomeRelief);
        }
        else if(biomeLoveFactor <= 0 && p.inHomeBiome)
        {
            p.inHomeBiome = false;
            //p.doEmote(Emote.biomeRelief);
        }

        if(p.getClosePlayer() != null && p.angryTime < 0)
        {
            //trace('SPEED HOSTLE NEAR');
            speed *= ServerSettings.CloseEnemyWithWeaponSpeedFactor;
        }

        //if(ServerSettings.DebugSpeed) trace('speed: $speed age: ${p.age} ageSpeedFactor: ${ageSpeedFactor} biomeSpeed: $biomeSpeed floorSpeed: $floorSpeed fullPathHasRoad:${fullPathHasRoad} speedModHeldObj: $speedModHeldObj Starving to death: ${p.food_store < 0}');
        if(ServerSettings.DebugSpeed) trace('speed: $speed age: ${p.age} biomeSpeed: $biomeSpeed floorSpeed: $floorSpeed fullPathHasRoad:${fullPathHasRoad} speedModHeldObj: $speedModHeldObj Starving to death: ${p.food_store < 0}');

        return speed;
    }
    
    static private function calculateObjSpeedMult(obj:ObjectHelper) : Float
    {
        return Math.max(0.6, Math.min(ServerSettings.MinSpeedReductionPerContainedObj, obj.objectData.speedMult)); 
    }

    public function isMoving()
    {
        return newMoves != null;        
    }

    /**
        Check if movement arrived on destination and if so update all players  
    **/
    static public function updateMovement(p:GlobalPlayerInstance)
    {
        var moveHelper = p.moveHelper;

        // test if not moving
        if(moveHelper.newMoves == null)
        {
            moveHelper.exactTx = p.tx;
            moveHelper.exactTy = p.ty;
            moveHelper.timeExactPositionChangedLast = TimeHelper.tick;
            return;
        } 

        // caclulate exact position. Important for combat vs player or animal // TODO use mutex if no global player mutex is used
        if(moveHelper.newMoves.length > 0)
        {
            var move = moveHelper.newMoves[0];
            var timePassed = TimeHelper.CalculateTimeSinceTicksInSec(moveHelper.timeExactPositionChangedLast); 
            var length = calculateLength(firstPos, move);
            var moved = timePassed * p.move_speed;

            moveHelper.exactTx = p.tx + (move.x * moved) / length;
            moveHelper.exactTy = p.ty + (move.y * moved) / length;

            // check if moved one step
            if(moved >= length)
            {
                move = moveHelper.newMoves.shift();
                p.x += move.x;
                p.y += move.y;          

                moveHelper.exactTx = p.tx;
                moveHelper.exactTy = p.ty;
                moveHelper.timeExactPositionChangedLast = TimeHelper.tick;

                for(pos in moveHelper.newMoves)
                {
                    pos.x -= move.x;
                    pos.y -= move.y;
                }

                if(p.heldPlayer != null)
                {
                    p.heldPlayer.x = p.tx - p.heldPlayer.gx;
                    p.heldPlayer.y = p.ty - p.heldPlayer.gy;
                }

                TimeHelper.MakeAnimalsRunAway(p);

                //if(ServerSettings.DebugMoveHelper) trace('Move: ${p.name} ${p.tx} ${p.ty}');
            }

            //if(TimeHelper.tick % 5 == 0) if(ServerSettings.DebugMoveHelper) trace('Moves: ${moveHelper.newMoves} passedTime: $timePassed ${p.tx()},${p.ty()} ${moveHelper.exactTx},${moveHelper.exactTy}');
        }

        var timeSinceStartMovementInSec = TimeHelper.CalculateTimeSinceTicksInSec(moveHelper.startingMoveTicks);

        timeSinceStartMovementInSec *= ServerSettings.LetTheClientCheatLittleBitFactor;

        //if(TimeHelper.tick % 5 == 0) if(ServerSettings.DebugMoveHelper) trace('Moves: timeSinceStartMovementInSec: ${timeSinceStartMovementInSec} totalMoveTime: ${moveHelper.totalMoveTime}');

        if(timeSinceStartMovementInSec >= moveHelper.totalMoveTime)
        {
            var last = moveHelper.newMoves.pop(); 
            moveHelper.totalMoveTime = 0;
            moveHelper.startingMoveTicks = 0;
            moveHelper.newMoves = null;
            moveHelper.newMovements = null;

            var oldX = p.x;
            var oldY = p.y;

            if(last != null)
            {
                p.x += last.x; 
                p.y += last.y;
            
                //if(p.connection.serverAi == null) if(ServerSettings.DebugMoveHelper) trace('reached position: g${p.tx},g${p.ty} FROM ${oldX},${oldY} TO ${p.x},${p.y}');
                //else if(ServerSettings.DebugMoveHelper) trace('AAI: GOTO: FROM ${oldX},${oldY} TO ${p.x},${p.y} / FROM g${p.tx - last.x},g${p.ty- last.y} TO g${p.tx},g${p.ty} reached position!');
            }

            p.done_moving_seqNum = moveHelper.newMoveSeqNumber;
            p.move_speed = calculateSpeed(p, p.x + p.gx, p.y + p.gy);

            if(p.heldPlayer != null)
            {
                p.heldPlayer.x = p.tx - p.heldPlayer.gx;
                p.heldPlayer.y = p.ty - p.heldPlayer.gy;
            }

            p.moveHelper.exactTx = p.tx;
            p.moveHelper.exactTy = p.ty;

            Connection.SendUpdateToAllClosePlayers(p);

            if(p.connection.serverAi != null) p.connection.serverAi.finishedMovement();

            //if(ServerSettings.DebugMoveHelper) trace('Move: ${p.p_id} ${p.name} ${p.tx} ${p.ty} Done SeqNum: ${p.done_moving_seqNum}');
        }
    }

    public function receivedForce(x:Int, y:Int)
    {
        if(player.x != x || player.y != y)
        {
            trace('WARNING: Force: Client: $x,$y != Server: ${player.x},${player.y}');
            return;
        }

        waitForForce = false;
    }

    /*
    PM
    p_id xs ys total_sec eta_sec trunc xdelt0 ydelt0 ... xdeltN ydeltN
    p_id xs ys total_sec eta_sec trunc xdelt0 ydelt0 ... xdeltN ydeltN
    ...
    p_id xs ys total_sec eta_sec trunc xdelt0 ydelt0 ... xdeltN ydeltN
    #

    List of player ids that just started moving, their start x y grid position,
    their delta grid offsets along their path (xs + xdelt0 = first destination x), 
    how long the total move should take (in case we 
    come into the game in the middle of a move), and their time to arrival in 
    floating point seconds

    trunc is 0 for untruncated path, or 1 for truncated path.
    Truncated paths are shorter than what the player originally requested.
    This can happen in immediate response to the move request or later, mid-move,
    if the path gets cut off (a new PM will be sent to truncate the path at that
    point)

    A PLAYER_UPDATE will be sent with done_moving set to 1 when these players 
    reach their destination.
    Until that has happened, client must assume player is still in transit.
    */
    static public function move(p:GlobalPlayerInstance, x:Int,y:Int,seq:Int,moves:Array<Pos>)
    {
        GlobalPlayerInstance.AllPlayerMutex.acquire();

        Macro.exception(moveHelper(p, x, y, seq, moves));

        GlobalPlayerInstance.AllPlayerMutex.release();
    }

    static private function JumpToNonBlocked(player:GlobalPlayerInstance) : Bool
    {
        var rand = 0;
        var tx = player.tx;
        var ty = player.ty;

        if(player.isBlocked(tx, ty) == false) return false; 
        
        for(i in 1...5)
        {
            var xo = 0;
            var yo = 0;

            if(rand == 1) xo = 1;
            if(rand == 2) yo = -1;
            if(rand == 3) xo = -1;
            if(rand == 4) yo = 1;

            rand++;

            if(player.isBlocked(tx + xo, ty + yo)) continue;

            player.x += xo;
            player.y += yo;
            player.exhaustion += 3;

            player.moveHelper.exactTx = player.tx;
            player.moveHelper.exactTy = player.ty;

            break;
        }

        return true;
    }
    
    static private function moveHelper(p:GlobalPlayerInstance, x:Int,y:Int,seq:Int,moves:Array<Pos>)
    {
        //trace("newMoveSeqNumber: " + newMoveSeqNumber);
        // dont accept moves untill a force is confirmed
        if(p.moveHelper.waitForForce)
        {
            trace('${p.name}: Move ignored since waiting for force!!');
            return;
        }

        if(p.age * 60 < ServerSettings.MinMovementAgeInSec)
        {
            p.done_moving_seqNum  = seq;
            p.connection.send(PLAYER_UPDATE,[p.toData()]);
            return;
        }

        if(p.isHeld()) p.jump();

        if(JumpToNonBlocked(p))
        {
            p.done_moving_seqNum  = seq;
            p.connection.send(PLAYER_UPDATE,[p.toData()]);
            return;
        }

        var jump = (p.x != x || p.y != y);
        var tx = x + p.gx;
        var ty = y + p.gy;
        var moveHelper = p.moveHelper;
        //var quadDist = p.moveHelper.calculateExactQuadDistance(tx, ty);
        var quadDist = jump ? p.moveHelper.calculateExactQuadDistance(tx, ty) : 0;

        if(p.isBlocked(tx,ty) || quadDist > ServerSettings.MaxMovementQuadJumpDistanceBeforeForce)
        {
            trace('${p.name} MOVE: FORCE!! Movement cancled since blocked or Client uses too different x,y: quadDist: $quadDist exact: ${Math.ceil((p.moveHelper.exactTx - p.gx) * 10) / 10},${Math.ceil((p.moveHelper.exactTy - p.gy) * 10) / 10} Server ${ p.x },${ p.y } <--> Client ${ x },${ y }');
            cancleMovement(p, seq);
            return;    
        }

        var positionChanged = false;

        if(jump)
        {
            //if(Math.ceil(p.jumpedTiles) >= ServerSettings.MaxJumpsPerTenSec || p.exhaustion > 5 + p.food_store_max / 2)
            if(Math.ceil(p.jumpedTiles) >= ServerSettings.MaxJumpsPerTenSec)
            {
                if(ServerSettings.DebugMoveHelper) trace('${p.name} MOVE: JUMP: FORCE!! Movement cancled since too exhausted ${Math.ceil(p.exhaustion)} or jumped: ${Math.ceil(p.jumpedTiles * 10) / 10} to often: quadDist: $quadDist Server ${ p.x },${ p.y } --> Client ${ x },${ y }');
                cancleMovement(p, seq);
                return;
            } 

            if(ServerSettings.DebugMoveHelper) trace('${p.name} MOVE: JUMP: positionChanged NoForce! jumped: ${Math.ceil(p.jumpedTiles * 10) / 10} quadDist: ${quadDist} Server ${ p.x },${ p.y } --> Client ${ x },${ y }');

            // Since vanilla client handels move strangely the server accepts one position further even if not fully reached there 
            // This a client could use to jump.
            // Therefore exhaustion and jumpedTiles is used to limit client jumps / "cheeting"      
            p.exhaustion += quadDist * ServerSettings.ExhaustionOnJump;
            p.jumpedTiles += p.exhaustion < p.food_store_max / 2 ? quadDist / 2 : quadDist;

            positionChanged = true;
            
            p.x = x;
            p.y = y;

            p.moveHelper.exactTx = p.tx;
            p.moveHelper.exactTy = p.ty;
        }

        // since it seems speed cannot be set for each tile, the idea is to cut the movement once it crosses in different biomes
        // if passing in an biome with different speed only the first movement is kept
        var newMovements = calculateNewMovements(p, moves);
        if(newMovements.moves.length < 1)
        {
            if(ServerSettings.DebugMoveHelper) trace('${p.name} MOVE: FORCE!! Move cancled since no new movements!');
            cancleMovement(p, seq);
            return;
        }
        
        p.move_speed = newMovements.finalSpeed;

        moveHelper.newMovements = newMovements;
        moveHelper.newMoves = newMovements.moves;
        moveHelper.totalMoveTime = (1 / p.move_speed) * newMovements.length;
        moveHelper.startingMoveTicks = TimeHelper.tick;
        moveHelper.newMoveSeqNumber = seq;  
        
        // TODO chunk loading in x direction is too slow with high speed
        // TODO general better chunk loading
        var spacingX = 4;
        var spacingY = 4;

        if(p.x - moveHelper.tx > spacingX || p.x - moveHelper.tx < -spacingX || p.y - moveHelper.ty > spacingY || p.y - moveHelper.ty < -spacingY )
        {          
            moveHelper.tx = p.x;
            moveHelper.ty = p.y;

            p.connection.sendMapChunk(p.x,p.y);         
        }                

        p.forced = false;

        if(positionChanged)
        {
            Connection.SendUpdateToAllClosePlayers(p);
        }
        else
        {
            //p.connection.sendPlayerUpdate();
        }

        Connection.SendMoveUpdateToAllClosePlayers(p);
    }     
    
    public static function cancleMovement(p:GlobalPlayerInstance, seq:Int)
    {
        if(p.isHuman()) p.moveHelper.waitForForce = true; // ignore all moves untill client sends a force

        p.moveHelper.exactTx = p.tx;
        p.moveHelper.exactTy = p.ty;
        p.done_moving_seqNum  = seq;
        p.move_speed = calculateSpeed(p, p.tx, p.ty);
        p.moveHelper.newMoves = null; 
        p.moveHelper.newMovements = null;
        p.forced = true;
        Connection.SendUpdateToAllClosePlayers(p);
        p.forced = false;
    }

    public function generateRelativeMoveUpdateString(forPlayer:GlobalPlayerInstance) : String
    {
        var totalMoveTime = Math.round(this.totalMoveTime * 100) / 100;
        var targetX = player.tx - forPlayer.gx;
        var targetY = player.ty - forPlayer.gy;
        var eta = totalMoveTime - TimeHelper.CalculateTimeSinceTicksInSec(startingMoveTicks);            

        var moveString = '${player.p_id} ${targetX} ${targetY} ${totalMoveTime} $eta ${newMovements.trunc} ${moveString(newMoves)}';
        //if(ServerSettings.DebugMoveHelper) trace('TEST Move: totalMoveTime: $totalMoveTime eta: $eta  $moveString');

        return moveString;
    }

    static private function moveString(moves:Array<Pos>):String
    {
        var string = "";
        for (m in moves) string += " " + m.x + " " + m.y;
        return string.substr(1);
    }

    static private function calculateLength(lastPos:Pos, pos:Pos):Float
    {
        // diagonal steps are longer
        if(lastPos.x != pos.x && lastPos.y != pos.y ){
            // diags are square root of 2 in length
            var diagLength = 1.41421356237; 
            return diagLength;
        }
        else {
            return 1;
        }
    }        

    // if path has a biome with different speed, path is trunced if movement is not on a road
    static private function calculateNewMovements(p:GlobalPlayerInstance, moves:Array<Pos>) : NewMovements 
    {
        var tx = p.tx;
        var ty = p.ty;
        var truncMovementSpeedDiff = 0.1;
        var newMovements:NewMovements = new NewMovements();
        var map = Server.server.map;
        var lastPos:Pos = new Pos(0,0);

        newMovements.fullPathHasRoad = true;
        newMovements.startSpeed = map.getBiomeSpeed(tx,ty);

        for (move in moves)
        {
            var tmpX = tx + move.x;
            var tmpY = ty + move.y;

            //var obj = WorldMap.world.getObjectHelper(tmpX, tmpY);
            //var isBlockingObj = obj.blocksWalking();
            //var isBlockingBiome = WorldMap.isBiomeBlocking(tmpX, tmpY);

            if(p.isBlocked(tmpX, tmpY))
            {
                //if(isBlockingBiome) if(ServerSettings.DebugMoveHelper) trace('biome ${map.getBiomeId(tmpX,tmpY)} is blocking movement! movement length: ${newMovements.length}');
                //if(isBlockingObj) if(ServerSettings.DebugMoveHelper) trace('object ${obj.description} is blocking movement! movement length: ${newMovements.length}');
                
                newMovements.trunc = 1;

                newMovements.finalSpeed = calculateSpeed(p, p.tx, p.ty, newMovements.fullPathHasRoad);

                return newMovements;
            }

            if(newMovements.fullPathHasRoad)
            {
                var floorObjData = ObjectData.getObjectData(map.getFloorId(tmpX,tmpY));
                if(floorObjData.speedMult < 1.01) newMovements.fullPathHasRoad = false;
            }

            newMovements.endSpeed = map.getBiomeSpeed(tmpX,tmpY); 

            if(newMovements.fullPathHasRoad == false && Math.pow(newMovements.endSpeed - newMovements.startSpeed, 2) > Math.pow(truncMovementSpeedDiff, 2))
            {                    
                /*if(newMovements.moves.length == 0)
                {
                    // dont cut the patch if one tile close to new biome
                    // TODO this may make problems, since client does now update move speed after move started
                    newMovements.startSpeed = newMovements.endSpeed;
                    //newMovements.length += calculateLength(lastPos,move);
                    //newMovements.moves.push(move);
                }*/
                
                
                //if(ServerSettings.DebugMoveHelper) trace('movement is trunc because of moving from bad biome to good biome or good biome to bad biome: ${newMovements.moves.length}');

                newMovements.length += calculateLength(lastPos,move);
                newMovements.moves.push(move);

                if(moves.length > 1) newMovements.trunc = 1;

                newMovements.finalSpeed = calculateSpeed(p, p.tx, p.ty, newMovements.fullPathHasRoad);

                return newMovements;
            }

            newMovements.length += calculateLength(lastPos,move);

            newMovements.moves.push(move);
            lastPos = move;
        }   

        newMovements.finalSpeed = calculateSpeed(p, p.tx, p.ty, newMovements.fullPathHasRoad);

        return newMovements;
    }      

    // this calculates which position is reached in case the movement was changed while moving
    static private function calculateNewPos(moves:Array<Pos>, startingMoveTicks:Float, speed:Float):Pos
    {
        var lastPos:Pos = new Pos(0,0);
        var length = 0.0;
        var timeSinceStartMovementInSec = TimeHelper.CalculateTimeSinceTicksInSec(startingMoveTicks);
        var movedLength = timeSinceStartMovementInSec * speed;
        
        // since client is some how faster allow client to chat little bit
        timeSinceStartMovementInSec *= ServerSettings.LetTheClientCheatLittleBitFactor; 

        for (move in moves)
        {
            var thisStepLength = calculateLength(lastPos,move);
            length += thisStepLength;
            //if(ServerSettings.DebugMoveHelper) trace('length: $length movedLength: $movedLength speed: $speed timeSinceStartMovementInSec: $timeSinceStartMovementInSec'  );
            
            // TODO make exact calculatation where the client thinks he is
            if(length - thisStepLength / 2 > movedLength) return lastPos;
            //if(length > movedLength) return lastPos;

            lastPos = move;
        }

        // in this case the whole movement finished 
        if(ServerSettings.DebugMoveHelper) trace("The whole movement finished");
        return lastPos;
    }


    /* pixel calulation stuff from Jason server.cpp
// never move at 0 speed, divide by 0 errors for eta times
    if( speed < 0.01 ) {
        speed = 0.01;
        }

    
    // after all multipliers, make sure it's a whole number of pixels per frame

    double pixelsPerFrame = speed * 128.0 / 60.0;
    
    
    if( pixelsPerFrame > 0.5 ) {
        // can round to at least one pixel per frame
        pixelsPerFrame = lrint( pixelsPerFrame );
        }
    else {
        // fractional pixels per frame
        
        // ensure a whole number of frames per pixel
        double framesPerPixel = 1.0 / pixelsPerFrame;
        
        framesPerPixel = lrint( framesPerPixel );
        
        pixelsPerFrame = 1.0 / framesPerPixel;
        }
    
    speed = pixelsPerFrame * 60 / 128.0;
        
    return speed;
    }

    */
}