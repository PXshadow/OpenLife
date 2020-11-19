package openlife.server;
import haxe.ds.Vector;
import openlife.data.object.ObjectHelper;
import openlife.data.map.MapData;
import openlife.data.transition.TransitionData;
import openlife.data.Pos;
import openlife.data.object.player.PlayerInstance;
import sys.thread.Mutex;

using openlife.server.MoveExtender;

class GlobalPlayerInstance extends PlayerInstance {
    // holds additional ObjectInformation for the object held in hand / null if there is no additional object data
    public var heldObject:ObjectHelper; 

    // additional ObjectInformation for the object stored in backpack or other clothing. The size is 6 since 6 clothing slots
    public var clothingObjects:Vector<ObjectHelper> = new Vector(6); 

    // handles all the movement stuff
    public var me:MoveExtender = new MoveExtender();
    // is used since move and move update can change the player at the same time
    public var mutux = new Mutex();

    public var connection:Connection; 

    // remember that y is counted from bottom not from top
    public var gx:Int = 400; //global x offset from birth
    public var gy:Int = 300; //global y offset from birth 

    public function new(a:Array<String>)
    {
        super(a);
    }

    public function isClose(x:Int, y:Int, distance:Int = 1):Bool{    
        return (((this.x - x) * (this.x - x) <= distance) && ((this.y - y) * (this.y - y) <= distance));
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
    public function self(x:Int, y:Int, clothingSlot:Int)
    {
        var doaction = false;
        var p_clothingSlot = -1;

        // TODO food on self

        if(this.o_id[0] != 0){
            var objectData = Server.objectDataMap[this.o_id[0]];
            //trace("OD: " + objectData.toFileString());

            if(objectData.clothing.charAt(0) == 'h'){
                p_clothingSlot = 0;
            }

            switch objectData.clothing.charAt(0) {
                case "h": p_clothingSlot = 0;
                case "t": p_clothingSlot = 1;
                case "s": p_clothingSlot = 2;
                //case "s": p_clothingSlot = 3; 
                case "b": p_clothingSlot = 4;
                case "p": p_clothingSlot = 5;
            }

            //trace('objectData.clothing: ${objectData.clothing}');
            //trace('p_clothingSlot:  ${p_clothingSlot}');
            //trace('clothingSlot:  ${clothingSlot}');
        }

        if(p_clothingSlot >= 0 || clothingSlot >=0){
            var array = this.clothing_set.split(";");

            if(array.length < 6){
                trace('Clothing string missing slots: ${this.clothing_set}' );
            }  

            // set  the index for shoes that come on the other feet
            if(p_clothingSlot == 2 && clothingSlot == -1){
                clothingSlot = 3;
            }else{
                clothingSlot = p_clothingSlot;
            }

            // TODO if the clothing are shoes and there are shoes allready on the first shoe but not on the second and if the index is not set
            
            if(clothingSlot >= 0){
                // switch clothing if there is a clothing on this slot
                var tmp = Std.parseInt(array[clothingSlot]);
                array[clothingSlot] = '${this.o_id[0]}';
                this.clothing_set = '${array[0]};${array[1]};${array[2]};${array[3]};${array[4]};${array[5]}';

                doaction = true;
                this.o_id = [tmp];
                this.action = 1;
                this.action_target_x = x;
                this.action_target_y = y;
                this.o_origin_x = x;
                this.o_origin_y = y;
                this.o_origin_valid = 0; // TODO ???

                //trace('this.clothing_set: ${this.clothing_set}');
            }

            //this.clothing_set = "0;0;0;0;0;0";
        }
        

        for (c in Server.server.connections) // TODO only for visible players
        {
            c.send(PLAYER_UPDATE,[this.toData()]);
            //if(doaction) c.sendMapUpdate(x,y,newFloorId, tile_o_id[0], this.p_id);
            c.send(FRAME);
        }

        this.action = 0;
    }
     
     public function remove(x:Int,y:Int,index:Int) : Bool
    {
        var helper = new TransitionHelper(this, x, y);

        helper.remove(index);
        
        return helper.sendUpdateToClient();
    }

    public function specialRemove(x:Int,y:Int,clothing:Int,id:Null<Int>)
    {
        for (c in Server.server.connections) // TODO only for visible players
            {
                c.send(PLAYER_UPDATE,[this.toData()]);
                c.send(FRAME);
            }
    }

    // even send Player Update / PU if nothing happend. Otherwise client will get stuck
    public function use(x:Int,y:Int) : Bool
    {
        var helper = new TransitionHelper(this, x, y);

        helper.use();

        return helper.sendUpdateToClient();
    }

    // even send Player Update / PU if nothing happend. Otherwise client will get stuck
    public function drop(x:Int,y:Int) : Bool
    {
        var helper = new TransitionHelper(this, x, y);
        
        if(helper.checkIfNotMovingAndCloseEnough() == false) return helper.sendUpdateToClient();

        helper.swapHandAndFloorObject();            
        
        return helper.sendUpdateToClient();
    }   
}