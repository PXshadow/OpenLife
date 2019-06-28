import motion.MotionPath;
import openfl.geom.Point;
import motion.Actuate;
import haxe.Timer;
import Display.Group;
import ObjectData.SpriteData;
class Player extends Group
{
    //done moving sequence number
    public var lastMove:Int = 1;
    public var head:Int = 0;
    public var body:Int = 0;
    public var backFoot:Array<Int> = [];
    public var frontFoot:Array<Int> = [];
    public var index:Int = 0;
    public var length:Int = 0;
    public var id:Int = 0;
    //object id, what is being held
    public var oid:Int = 0;
    public var objectGroup:Group;
    //id refrence to renderMap | player object id
    public var poid:Int = 0;
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
    public var tileX:Int = 0;
    public var tileY:Int = 0;
    public var moveTimer:Timer;
    //mouth and face
    var mainEyesOffset:Point = new Point(0,0);
    public function new(id:Int,tileX:Int,tileY:Int)
    {
        super();
        this.id = id;
        this.tileX = tileX;
        this.tileY = tileY;
        //trace("starting pos " + tileX + " " + tileY);
        //set start pos
        x = tileX * Static.GRID;
        y = tileY * Static.GRID;
        active.set(id,this);
    }
    public function setupeyesAndMouth()
    {
        
    }
    public function ageSystem(rate:Float)
    {
        /*var timer = new Timer(1000 * rate);
        timer.run = function()
        {
            //age += 1;
            agePlayer();
        }*/
    }
    public function update()
    {
        
    }
    public function move(moveX:Int=0,moveY:Int=0)
    {
        if (moveTimer != null || moveX == 0 && moveY == 0) return;
        moveTimer = new Timer(300 * 1);
        moveTimer.run = function()
        {
            moveTimer.stop();
            moveTimer = null;
        }
        lastMove++;
        Main.client.send("MOVE " + tileX + " " + tileY + " @" +
        lastMove + " " +
        moveX + " " + moveY
        );
        tileX += moveX;
        tileY += moveY;
        Actuate.tween(this,0.4,{x: tileX * Static.GRID,y: -tileY * Static.GRID});
        //floor
        var floor = Main.client.map.floor.get(tileX + "." + tileY);
    }
    public function use(offsetX:Int=0,offsetY:Int=0)
    {
        offsetX += tileX;
        offsetY += tileY;
        var obj = Main.client.map.object.get(offsetX + "." + offsetY);
        oid = Std.parseInt(obj);
        Main.client.send("USE " + offsetX + " " + offsetY);
    }
    public function drop(offsetX:Int=0,offsetY:Int=0,c:Int=-1)
    {
        Main.client.send("DROP " + Std.string(tileX + offsetX) + " " + Std.string(tileY + offsetY) + " " + c);
        oid = 0;
    }
    public function agePlayer()
    {
        if(Display.renderMap.exists(poid))
        {
            var j:Int = 0;
            var array = Display.renderMap.get(poid);
            var sprite:SpriteData;
            for(i in 0...length)
            {
                sprite = array[j++];
                //reset visibility
                children[i].visible = true;
                if((sprite.ageRange[0] > age || sprite.ageRange[1] < age) && sprite.ageRange[0] > 0)
                {
                    //outside of range of age set invisible
                    children[i].visible = false;
                }
            }
            //shrink body
        }else{
            trace("player rendermap object not found");
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