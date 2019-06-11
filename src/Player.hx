import haxe.Timer;
import Display.Group;
import ObjectData.SpriteData;
class Player extends Group
{
    //done moving sequence number
    public static var lastMoveSequenceNumber:Int = 1;
    public var head:Int = 0;
    public var body:Int = 0;
    public var backFoot:Array<Int> = [];
    public var frontFoot:Array<Int> = [];
    public var index:Int = 0;
    public var length:Int = 0;
    public var id:Int = 0;
    //object id, what is being held
    public var oid:Int = 0;
    //id refrence to renderMap
    public var pid:Int = 0;
    public var age:Int = 0;
    public var speed:Float = 0;
    //clothing
    public var hat:Int = 0;
    public var tunic:Int = 0;
    public var front_shoe:Int = 0;
    public var back_shoe:Int = 0;
    public var bottom:Int = 0;
    public var backpack:Int = 0;
    public static var active:Map<Int,Player> = new Map<Int,Player>();
    public static var main:Player;
    //movement
    var tileX:Int = 0;
    var tileY:Int = 0;
    var moveX:Int = 0;
    var moveY:Int = 0;
    var moveActive:Bool = false;
    var movePath:
    public function new(id:Int,tileX:Int,tileY:Int)
    {
        super();
        this.id = id;
        this.tileX = tileX;
        this.tileY = tileY;
        //set start pos
        x = tileX * Static.GRID;
        y = tileY * Static.GRID;
        if(main == null) main = this;
        active.set(id,this);
    }
    public function ageSystem(rate:Float)
    {
        var timer = new Timer(1000 * rate);
        timer.run = function()
        {
            age += 1;
            agePlayer();
        }
    }
    public function move(moveX:Int,moveY:Int)
    {
        if(tileX == moveX && tileY == moveY) return;
        this.moveX = moveX;
        this.moveY = moveY;
        moveActive = true;
    }
    public function movePath(movePath:Array<MovePath>)
    {
        //send to server
        if (main != this) return;
        Main.client.send("MOVE " + tileX + " " + tileY + );
    }
    public function update()
    {
        if(moveActive)
        {
            var sx = Math.floor(x/Static.GRID);
            var sy = Math.floor(y/Static.GRID);
            var speed = (speed * Static.GRID)/60;
            if(sx == moveX && sy == moveY)
            {
                //do things
                moveActive = false;
            }
            if (sx < moveX) x += speed;
            if (sx > moveX) x += -speed;
            if (sy < moveY) y += speed;
            if (sy > moveY) y += -speed;
        }
    }
    public function agePlayer()
    {
        //return trace("hi");
        age = 40;
        if(Display.renderMap.exists(pid))
        {
            var j:Int = 0;
            var array = Display.renderMap.get(pid);
            var sprite:SpriteData;
            for(i in 0...length)
            {
                sprite = array[j++];
                if((sprite.ageRange[0] > age || sprite.ageRange[1] < age) && sprite.ageRange[0] > 0)
                {
                    //outside of range of age
                    children[i].visible = false;
                    //getTileAt(i).visible = false;
                }
            }
        }else{
            throw("player rendermap object not found");
        }
    }
    public function setSection(index:Int,length:Int)
    {
        this.index = index;
        this.length = length;
    }
    public function unactive()
    {
        active.remove(id);
    }
}