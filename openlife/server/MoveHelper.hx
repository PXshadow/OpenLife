package openlife.server;
import haxe.macro.Expr.Catch;
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
    
    public function new() {

    }
}


class MoveHelper
{

    // x,y when last chunk was send
    private var tx:Int = 0;
    private var ty:Int = 0;

    // to calculate if the move is finished
    private var newMoveSeqNumber:Int = 0; 
    private var newMoves:Array<Pos>;
    private var totalMoveTime:Float = 0;
    private var startingMoveTicks:Float = 0;

    public function new(){}

    public function isMoveing():Bool
    {    
        return (this.newMoves != null);
    }

    static public function calculateSpeed(p:GlobalPlayerInstance, tx:Int, ty:Int, fullPathHasRoad:Bool = true) : Float
    {
        // TODO reduce speed for buckets depending on how full they are

        var map =  Server.server.map;
        var onHorseOrCar = p.heldObject.objectData.speedMult >= 1.1;
        var speed = ServerSettings.InitialPlayerMoveSpeed;
        var floorObjData = ObjectData.getObjectData(map.getFloorId(tx,ty));
        var floorSpeed = floorObjData.speedMult;
        var onRoad = false;
        var hasBothShoes = p.hasBothShoes();        

        if(ServerSettings.DebugSpeed) trace('speed: hasBothShoes: $hasBothShoes');

        if(hasBothShoes && onHorseOrCar == false) speed *= 1.1;

        if(fullPathHasRoad == false) floorSpeed = 1; // only consider road if the pull path is on road

        onRoad = floorSpeed >= 1.01; // only give road speed boni if full path is on road
        
        speed *= ServerSettings.SpeedFactor; // used to increase speed if for example debuging

        speed *= floorSpeed;



        // DO biomes
        var biomeSpeed = map.getBiomeSpeed(tx,ty);  

        // road reduces speed mali of bad biome with sqrt 
        if(onRoad && biomeSpeed < 0.99) biomeSpeed = 1; //biomeSpeed = Math.sqrt(biomeSpeed);

        speed *= biomeSpeed;



        // DO speed held objects
        var speedModHeldObj = p.heldObject.objectData.speedMult;

        if(biomeSpeed < 0.9 && speedModHeldObj > 1) // horses and cars are bad in bad biome 
        {
            if(speedModHeldObj > 2.50) speedModHeldObj = 0.5; // super speedy stuff like cars
            else if(speedModHeldObj > 1.8) speedModHeldObj = 0.8; // for example horse
            else if(speedModHeldObj > 1.2) speedModHeldObj = 0.6; // for example horse cart
            
            if(ServerSettings.DebugSpeed) trace('Speed: New ${p.heldObject.objectData.description} speed in bad biome: ${p.heldObject.objectData.speedMult} --> $speedModHeldObj');
        }
        
        if(onRoad && speedModHeldObj < 0.99) speedModHeldObj = Math.sqrt(speedModHeldObj); // on road
        speed *= speedModHeldObj;



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



        // DO starving speed
        if(p.food_store < 0 && onHorseOrCar == false) // only reduce speed when starving if not riding or in car 
        {
            if(p.yum_multiplier > 0) speed *= ServerSettings.StarvingToDeathMoveSpeedFactorWhileHealthAboveZero;
            else speed *= ServerSettings.StarvingToDeathMoveSpeedFactor;
        }


        // Do health speed
        var healthFactor = p.CalculateHealthFactor(true);

        if(ServerSettings.DebugSpeed) trace('speed: healthFactor: $healthFactor health: ${p.yum_multiplier} trueAge: ${p.trueAge} expected health: ${p.trueAge  * ServerSettings.MinHealthPerYear}');

        speed *= healthFactor;

        var ageSpeedFactor:Float = 1;



        // Do age speed
        if(p.age < 1) ageSpeedFactor = 0.5;
        else if(p.age < 2) ageSpeedFactor = 0.6; 
        else if(p.age < 3) ageSpeedFactor = 0.7;
        else if(p.age < 6) ageSpeedFactor = 0.8;
        else if(p.age < 10) ageSpeedFactor = 0.9;
        else if(p.age > 55) ageSpeedFactor = 0.8;
        
        /*
        if(p.age < 1) ageSpeedFactor = 0.5;
        else if(p.age < 2) ageSpeedFactor = 0.7; 
        else if(p.age < 3) ageSpeedFactor = 0.8;
        else if(p.age < 6) ageSpeedFactor = 0.9;
        else if(p.age < 10) ageSpeedFactor = 0.95;
        else if(p.age > 55) ageSpeedFactor = 0.8;
        */

        speed *= ageSpeedFactor;

        if(ServerSettings.DebugSpeed) trace('speed: $speed age: ${p.age} ageSpeedFactor: ${ageSpeedFactor} biomeSpeed: $biomeSpeed floorSpeed: $floorSpeed fullPathHasRoad:${fullPathHasRoad} speedModHeldObj: $speedModHeldObj Starving to death: ${p.food_store < 0}');

        return speed;
    }
    
    static private function calculateObjSpeedMult(obj:ObjectHelper) : Float
    {
        return Math.max(0.6, Math.min(ServerSettings.MinSpeedReductionPerContainedObj, obj.objectData.speedMult)); 
    }

    /**
        Check if movement arrived on destination and if so update all players  
    **/
    static public function updateMovement(p:GlobalPlayerInstance)
    {
        var moveHelper = p.moveHelper;

        if(moveHelper.newMoves == null) return;
        
        var timeSinceStartMovementInSec = TimeHelper.CalculateTimeSinceTicksInSec(moveHelper.startingMoveTicks);

        timeSinceStartMovementInSec *= ServerSettings.LetTheClientCheatLittleBitFactor;

        if(timeSinceStartMovementInSec >= moveHelper.totalMoveTime)
        {
            // a new move or command might also change the player data
            p.mutex.acquire(); 
            
            if(moveHelper.newMoves == null){p.mutex.release();  return;} // to be sure that meanwhile no other thread messed around

            //WorldMap.world.mutex.acquire(); // make sure that no other threat uses connections TODO change

            try
            {
                var last = moveHelper.newMoves.pop(); 
                moveHelper.totalMoveTime = 0;
                moveHelper.startingMoveTicks = 0;
                moveHelper.newMoves = null;
                    
                p.x += last.x; 
                p.y += last.y;
                
                p.done_moving_seqNum = moveHelper.newMoveSeqNumber;
                p.move_speed = calculateSpeed(p, p.x + p.gx, p.y + p.gy);

                //p.forced = true; // TODO try out

                Connection.SendUpdateToAllClosePlayers(p);
            }
            catch(ex)
            {
                trace(ex.details);
            }

            p.mutex.release();
            //WorldMap.world.mutex.release();

            trace('reached position: ${p.x},${p.y}');
        }
    }

    static public function move(p:GlobalPlayerInstance, x:Int,y:Int,seq:Int,moves:Array<Pos>)
        {
            //trace(Server.server.map.getObjectId(p.gx + x, p.gy + y));

            // since move update may acces this also
            p.mutex.acquire(); 

            try
            {
                var moveHelper = p.moveHelper;

                if(moveHelper.newMoves != null)
                {
                    var lastPos = calculateNewPos(moveHelper.newMoves, moveHelper.startingMoveTicks, p.move_speed);

                    p.x += lastPos.x;
                    p.y += lastPos.y;

                    //trace('LastPos ${ lastPos.x } ${ lastPos.y }');
                }

                // TODO dont accept moves untill a force is confirmed
                // TODO it accepts one position further even if not fully reached there. 
                // TODO maybe make player "exhausted" with lower movementspeed if he "cheats" to much
                // This could be miss used to double movement speed. But Client seems to do it this way...

                var biomeSpeed = Server.server.map.getBiomeSpeed(x + p.gx, y + p.gy);
                var isBlockingBiome = biomeSpeed < 0.1;

                if(isBlockingBiome || p.isClose(x,y,ServerSettings.MaxMovementCheatingDistanceBeforeForce) == false)
                {
                    p.forced = true;
                    p.done_moving_seqNum  = seq;
                    moveHelper.newMoves = null; // cancle all movements

                    trace('MOVE: Force!   Server ${ p.x },${ p.y }:Client ${ x },${ y }');
                }
                else
                {
                    if(p.x != x && p.y != y) trace('MOVE: NoForce! Server ${ p.x },${ p.y }:Client ${ x },${ y }');

                    p.forced = false;

                    p.x = x;
                    p.y = y;
                }

                //trace("newMoveSeqNumber: " + newMoveSeqNumber);
        
                // since it seems speed cannot be set for each tile, the idea is to cut the movement once it crosses in different biomes
                // TODO maybe better to not cut it and make a player update one the new biome is reached?
                // if passing in an biome with different speed only the first movement is kept
                var newMovements = calculateNewMovements(p, p.x + p.gx, p.y + p.gy, moves);

                if(newMovements.moves.length < 1)
                {
                    p.done_moving_seqNum  = seq;
                    p.move_speed = calculateSpeed(p, p.tx(), p.ty());
                    moveHelper.newMoves = null; // cancle all movements

                    //cancle movement
                    p.forced = true;
                    p.connection.send(PLAYER_UPDATE,[p.toData()]);
                    p.connection.send(FRAME);

                    
                    p.forced = false;
                    p.mutex.release();

                    return;
                }
                
                p.move_speed = newMovements.finalSpeed;

                moveHelper.newMoves = newMovements.moves;
                moveHelper.totalMoveTime = (1/p.move_speed) * newMovements.length;
                moveHelper.startingMoveTicks = TimeHelper.tick;
                moveHelper.newMoveSeqNumber = seq;  
                var eta = moveHelper.totalMoveTime;
                
                // TODO chunk loading in x direction is too slow with high speed
                // TODO general better chunk loading
                var spacing = 4;
        
                if(p.x - moveHelper.tx> spacing || p.x - moveHelper.tx < -spacing || p.y - moveHelper.ty > spacing || p.y - moveHelper.ty < -spacing )
                {          
                    moveHelper.tx = p.x;
                    moveHelper.ty = p.y;
        
                    p.connection.sendMapChunk(p.x,p.y);
                }


                p.connection.send(PLAYER_UPDATE,[p.toData()]);

                //c.send(PLAYER_MOVES_START,['${player.p_id} ${targetX} ${targetY} ${player.moveHelper.totalMoveTime} $eta ${newMovements.trunc} ${player.moveHelper.moveString(player.moveHelper.newMoves)}']);

                Connection.SendMoveUpdateToAllClosePlayers(p, moveHelper.totalMoveTime, newMovements.trunc, moveString(moveHelper.newMoves));

            }
            catch(ex)
            {
                trace(ex.details);
            }

            p.forced = false;
            p.mutex.release();
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
        static private function calculateNewMovements(p:GlobalPlayerInstance, tx:Int,ty:Int,moves:Array<Pos>):NewMovements 
        {
            var newMovements:NewMovements = new NewMovements();
            var map = Server.server.map;
            var lastPos:Pos = new Pos(0,0);
            newMovements.fullPathHasRoad = true;
            
            newMovements.startSpeed = map.getBiomeSpeed(tx,ty);
                        
            for (move in moves)
            {
                var tmpX = tx + move.x;
                var tmpY = ty + move.y;

                // check if biome is not walkable
                if(map.getBiomeSpeed(tmpX,tmpY) < 0.1)
                {
                    trace('biome ${map.getBiomeId(tmpX,tmpY)} is blocking movement! movement length: ${newMovements.length}');
                    
                    newMovements.trunc = 1;

                    newMovements.finalSpeed = calculateSpeed(p, p.tx(), p.ty(), newMovements.fullPathHasRoad);

                    return newMovements;
                }

                if(newMovements.fullPathHasRoad)
                {
                    var floorObjData = ObjectData.getObjectData(map.getFloorId(tmpX,tmpY));
                    if(floorObjData.speedMult < 1.01) newMovements.fullPathHasRoad = false;
                }

                newMovements.endSpeed = map.getBiomeSpeed(tmpX,tmpY); 

                if(newMovements.fullPathHasRoad == false && newMovements.endSpeed != newMovements.startSpeed)
                {                    
                    /*if(newMovements.moves.length == 0)
                    {
                        // dont cut the patch if one tile close to new biome
                        // TODO this may make problems, since client does now update move speed after move started
                        newMovements.startSpeed = newMovements.endSpeed;
                        //newMovements.length += calculateLength(lastPos,move);
                        //newMovements.moves.push(move);
                    }*/
                    
                    
                    trace('movement is trunc because of moving from bad biome to good biome or good biome to bad biome: ${newMovements.moves.length}');

                    newMovements.length += calculateLength(lastPos,move);
                    newMovements.moves.push(move);

                    if(moves.length > 1) newMovements.trunc = 1;

                    newMovements.finalSpeed = calculateSpeed(p, p.tx(),p.ty(), newMovements.fullPathHasRoad);

                    return newMovements;
                }

                newMovements.length += calculateLength(lastPos,move);

                newMovements.moves.push(move);
                lastPos = move;
            }       

            newMovements.finalSpeed = calculateSpeed(p, p.tx(), p.ty(), newMovements.fullPathHasRoad);

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
                //trace('length: $length movedLength: $movedLength speed: $speed timeSinceStartMovementInSec: $timeSinceStartMovementInSec'  );
                
                // TODO make exact calculatation where the client thinks he is
                if(length - thisStepLength / 2 > movedLength) return lastPos;
                //if(length > movedLength) return lastPos;

                lastPos = move;
            }

            // in this case the whole movement finished 
            trace("The whole movement finished");
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