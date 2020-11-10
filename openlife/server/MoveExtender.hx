package openlife.server;
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


class MoveExtender{

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

    static public function move(p:GlobalPlayerInstance, x:Int,y:Int,seq:Int,moves:Array<Pos>)
        {
            trace('Server ${ p.x },${ p.y }:Client ${ x },${ y }');

            
            var me = p.me;
    
            //trace("newMoveSeqNumber: " + newMoveSeqNumber);
    
            // TODO what to do if old movement is not finished?
            
            //var last = moves.pop(); 
            //this.x += last.x; // TODO check if client movement is valid
            //this.y += last.y;
    
            p.x = x;
            p.y = y;
    
            // since it seems speed cannot be set for each tile, the idea is to cut the movement once it crosses in different biomes
            // TODO maybe better to not cut it and make a player update one the new biome is reached?
            // if passing in an biome with different speed only the first movement is kept
            var newMovements = calculateNewMovements(p.x + p.gx, p.y + p.gy, moves);
            if(newMovements.moves.length < 1) throw "newMoves.length < 1";
            
            // in case the new field has another speed take the lower (average) speed
            //speed = (speed + startSpeed) / 2; 
            if(newMovements.endSpeed < newMovements.startSpeed) newMovements.startSpeed = newMovements.endSpeed;
    
            newMovements.startSpeed *= PlayerInstance.initial_move_speed;
            //trace("speed:" + speed);
            var speedChanged = (p.move_speed != newMovements.startSpeed);
            
            // since move update may acces this also
            p.mutux.acquire(); 

            p.move_speed = newMovements.startSpeed;

            me.newMoves = newMovements.moves;
            me.totalMoveTime = (1/p.move_speed) * newMovements.length;
            me.startingMoveTicks = Server.server.tick;
            me.newMoveSeqNumber = seq;

            p.mutux.release();
            
            var eta = me.totalMoveTime;
            //p.done_moving_seqNum = 0;
            
            // TODO spacing / chunk loading in x direction is too slow with high speed
            // TODO general better chunk loading
            var spacing = 4;
    
            if(p.x - me.tx> spacing || p.x - me.tx < -spacing || p.y - me.ty > spacing || p.y - me.ty < -spacing ) {          
                
                me.tx = p.x;
                me.ty = p.y;
    
                p.connection.sendMapChunk(p.x,p.y);
            }
    
            // TODO there is a bug with speed update 
            if(speedChanged) p.connection.send(PLAYER_UPDATE,[p.toData()]);
    
            for (c in Server.server.connections) 
            {
                c.send(PLAYER_MOVES_START,['${p.p_id} $x $y ${me.totalMoveTime} $eta ${newMovements.trunc} ${moveString(me.newMoves)}']);
                
                c.send(FRAME);
            }
        }

        static private function calculateNewMovements(tx:Int,ty:Int,moves:Array<Pos>):NewMovements 
        {
            var newMovements:NewMovements = new NewMovements();
            var map =  Server.server.map;
            var lastPos:Pos = new Pos(0,0);
            
            newMovements.startSpeed = map.getBiomeSpeed(tx,ty);
            
            for (move in moves) {

                newMovements.endSpeed = map.getBiomeSpeed(tx + move.x,ty + move.y);

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

        static private function moveString(moves:Array<Pos>):String
        {
            var string = "";
            for (m in moves) string += " " + m.x + " " + m.y;
            return string.substr(1);
        }
    
        private static function calculateLength(lastPos:Pos, pos:Pos):Float
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

        static public function updateMovement(p:GlobalPlayerInstance)
        {
            var me = p.me;
            // check if movement arrived on destination and if so update all players  
            var server = Server.server;
            var timeSinceLastMovementInSec = (server.tick - me.startingMoveTicks) * Server.tickTime;
    
            if(me.newMoves == null) return;
    
            if(server.tick % 60 == 100){
                trace("Ticks: " + server.tick);
                trace("timeSinceLastMovementInSec: " + timeSinceLastMovementInSec);
                trace("totalMoveTime: " + me.totalMoveTime);
            }
    
            if(timeSinceLastMovementInSec >= me.totalMoveTime){

                // a new move or command might also change the player data
                p.mutux.acquire(); 

                var last = me.newMoves.pop(); 
                me.totalMoveTime = 0;
                me.startingMoveTicks = 0;
                me.newMoves = null;
                   
                p.x += last.x; 
                p.y += last.y;
                
                p.done_moving_seqNum = me.newMoveSeqNumber;
                p.move_speed = server.map.getBiomeSpeed(p.x + p.gx, p.y + p.gy) * PlayerInstance.initial_move_speed;
                //this.forced = true;
    
                p.mutux.release();
    
                //trace('reached position: ${p.x},${p.y}');
             
                //trace("forced: " + p.forced);
    
                for (c in Server.server.connections) 
                {
                    //c.send(PLAYER_MOVES_START,['${this.p_id} $x $y $totalMoveTime $totalMoveTime $trunc ${moveString(newMoves)}']);
                    c.send(PLAYER_UPDATE,[p.toData()]);
                    c.send(FRAME);
                }
            }
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
    /*
    static private function sendSpeedUpdate(c:Connection)
    {
        var speed = PlayerInstance.initial_move_speed * Server.server.map.getBiomeSpeed(p.x + p.gx, p.y + p.gy);

        //trace("speed: " + speed);

        //this.move_speed = 10;

        p.move_speed = speed;
        
        // TODO place sending logic in connection???
        c.send(PLAYER_UPDATE,[p.toData()]);
    }   */
}