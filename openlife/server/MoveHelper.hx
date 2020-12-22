package openlife.server;
import haxe.Serializer;
import openlife.settings.ServerSettings;
import openlife.data.object.player.PlayerInstance;
import openlife.data.Pos;

//@:multiReturn extern class NewMovements {
private class NewMovements {
    public var moves:Array<Pos> = [];
    public var length:Float;
    // speed of starting Tile
    public var startSpeed:Float;
    // speed of last Movement Tile
    public var endSpeed:Float;
    // true if movement was cut
    public var trunc:Int;
    
    public function new() {

    }
}


class MoveHelper{

    // x,y when last chunk was send
    private var tx:Int = 0;
    private var ty:Int = 0;

    // to calculate if the move is finished
    private var newMoveSeqNumber:Int = 0; 
    private var newMoves:Array<Pos>;
    private var totalMoveTime:Float = 0;
    private var startingMoveTicks:Int = 0;

    public function new(){
    }

    public function isMoveing():Bool{    
        return (this.newMoves != null);
    }

    static public function calculateSpeed(p:GlobalPlayerInstance, tx:Int,ty:Int) : Float
    {
        var map =  Server.server.map;

        // TODO heavy objects in chart

        // TODO chart off road

        // TODO shoes extra speed

        // TODO road 

        // TODO health        

        var speed = ServerSettings.InitialPlayerMoveSpeed;

        speed *= ServerSettings.SpeedFactor; // used to increase speed if for example debuging

        var biomeSpeed = map.getBiomeSpeed(tx,ty);

        speed *= biomeSpeed;

        var speedModHeldObj = p.heldObject.objectData.speedMult;

        speed *= speedModHeldObj;

        // only reduce speed when starving if not riding or in car 
        if(p.food_store < 0 && p.heldObject.objectData.speedMult < 1.1) speed *= ServerSettings.StarvingToDeathMoveSpeedFactor;

        var healthFactor = p.CalculateHealthFactor(true);

        trace('speed: healthFactor: $healthFactor health: ${p.yum_multiplier}');

        speed *= healthFactor;

        var ageSpeedFactor:Float = 1;

        // may not be needed, since speed is slowed down because of bad YUM / health in the beginning
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

        trace('speed: $speed age: ${p.age} ageSpeedFactor: ${ageSpeedFactor} biomeSpeed: $biomeSpeed speedModHeldObj: $speedModHeldObj Starving to death: ${p.food_store < 0}');

        return speed;
    }

    static public function updateMovement(p:GlobalPlayerInstance)
    {
        var moveHelper = p.moveHelper;
        // check if movement arrived on destination and if so update all players  
        //var server = Server.server;
        var timeSinceStartMovementInSec = TimeHelper.CalculateTimeSinceTicksInSec(moveHelper.startingMoveTicks);

        if(moveHelper.newMoves == null) return;

        /*
        if(server.tick % 60 == 0){
            trace("Ticks: " + server.tick);
            trace("timeSinceStartMovementInSec: " + timeSinceStartMovementInSec);
            trace("totalMoveTime: " + me.totalMoveTime);
        }
        */

        if(timeSinceStartMovementInSec >= moveHelper.totalMoveTime){

            // a new move or command might also change the player data
            p.mutex.acquire(); 

            var last = moveHelper.newMoves.pop(); 
            moveHelper.totalMoveTime = 0;
            moveHelper.startingMoveTicks = 0;
            moveHelper.newMoves = null;
                
            p.x += last.x; 
            p.y += last.y;
            
            p.done_moving_seqNum = moveHelper.newMoveSeqNumber;
            p.move_speed = calculateSpeed(p, p.x + p.gx, p.y + p.gy);
            //this.forced = true;

            trace('reached position: ${p.x},${p.y}');
            
            //trace("forced: " + p.forced);

            Connection.SendUpdateToAllClosePlayers(p);

            p.mutex.release();
        }
    }

    static public function move(p:GlobalPlayerInstance, x:Int,y:Int,seq:Int,moves:Array<Pos>)
        {
            //trace(Server.server.map.getObjectId(p.gx + x, p.gy + y));

            // since move update may acces this also
            p.mutex.acquire(); 

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
                moveHelper.newMoves = null; // cancle all movements

                //cancle movement
                p.forced = true;
                p.connection.send(PLAYER_UPDATE,[p.toData()]);
                p.connection.send(FRAME);

                
                p.forced = false;
                p.mutex.release();

                return;
                // throw "newMoves.length < 1";
            }
            
            // in case the new field has another speed take the lower (average) speed
            //speed = (speed + startSpeed) / 2; 
            if(newMovements.endSpeed < newMovements.startSpeed) newMovements.startSpeed = newMovements.endSpeed;
                
            //trace("speed:" + speed);
            //var speedChanged = (p.move_speed != newMovements.startSpeed);
            

            p.move_speed = newMovements.startSpeed;

            moveHelper.newMoves = newMovements.moves;
            moveHelper.totalMoveTime = (1/p.move_speed) * newMovements.length;
            moveHelper.startingMoveTicks = TimeHelper.tick;
            moveHelper.newMoveSeqNumber = seq;  
            
            var eta = moveHelper.totalMoveTime;
            //p.done_moving_seqNum = 0;
            
            // TODO spacing / chunk loading in x direction is too slow with high speed
            // TODO general better chunk loading
            var spacing = 4;
    
            if(p.x - moveHelper.tx> spacing || p.x - moveHelper.tx < -spacing || p.y - moveHelper.ty > spacing || p.y - moveHelper.ty < -spacing ) {          
                
                moveHelper.tx = p.x;
                moveHelper.ty = p.y;
    
                p.connection.sendMapChunk(p.x,p.y);
            }
    
            // TODO there is a bug with speed update 
            // TODO send PU to all players?
            p.connection.send(PLAYER_UPDATE,[p.toData()]);
    
            for (c in Server.server.connections) 
            {
                var targetX = x + p.gx - c.player.gx;
                var targetY = y + p.gy - c.player.gy;

                // update only close players
                if(c.player.isClose(targetX,targetY, ServerSettings.maxDistanceToBeConsideredAsClose) == false) continue;

                c.send(PLAYER_MOVES_START,['${p.p_id} ${targetX} ${targetY} ${moveHelper.totalMoveTime} $eta ${newMovements.trunc} ${moveString(moveHelper.newMoves)}']);
                
                c.send(FRAME);
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

        static private function calculateNewMovements(p:GlobalPlayerInstance, tx:Int,ty:Int,moves:Array<Pos>):NewMovements 
        {
            var newMovements:NewMovements = new NewMovements();
            var map = Server.server.map;
            var lastPos:Pos = new Pos(0,0);
            
            newMovements.startSpeed = calculateSpeed(p, tx,ty);
            
            for (move in moves) {
                // check if biome is not walkable
                if(map.getBiomeSpeed(tx + move.x,ty + move.y) < 0.1)
                {
                    trace('biome ${map.getBiomeId(tx + move.x,ty + move.y)} is blocking movement! movement length: ${newMovements.length}');
                    
                    newMovements.trunc = 1;
                    //newMovements.length = 0;
                    //newMovements.moves = new Array<Pos>();
                    return newMovements;
                }

                newMovements.endSpeed = calculateSpeed(p, tx + move.x,ty + move.y);

                

                if(newMovements.endSpeed != newMovements.startSpeed) {
                    if(newMovements.moves.length == 0){
                        newMovements.length += calculateLength(lastPos,move);
                        newMovements.moves.push(move);
                    }

                    if(moves.length > 1) newMovements.trunc = 1;
                    return newMovements;
                }

                newMovements.length += calculateLength(lastPos,move);

                newMovements.moves.push(move);
                lastPos = move;

            }

            return newMovements;
        }      

        // this calculates which position is reached in case the movement was changed while moving
        static private function calculateNewPos(moves:Array<Pos>, startingMoveTicks:Int, speed:Float):Pos
        {
            var timeSinceStartMovementInSec = TimeHelper.CalculateTimeSinceTicksInSec(startingMoveTicks);
            var movedLength = timeSinceStartMovementInSec * speed;
            var lastPos:Pos = new Pos(0,0);
            var length = 0.0;

            for (move in moves) {
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