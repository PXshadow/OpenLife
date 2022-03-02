import openlife.data.map.MapChange;
import openlife.data.map.MapInstance;
import openlife.data.object.player.PlayerInstance;
import openlife.data.object.player.PlayerMove;
import openlife.engine.EngineHeader;

class World implements EngineHeader {
	public function new() {}

	public function playerUpdate(instances:Array<PlayerInstance>) {}

	public function playerMoveStart(move:PlayerMove) {}

	public function playerOutOfRange(list:Array<Int>) {}

	public function playerName(id:Int, firstName:String, lastName:String) {}

	public function apocalypse() {}

	public function apocalypseDone() {}

	public function posse(killer:Int, target:Int) {}

	public function following(follower:Int, leader:Int, color:Int) {}

	public function exiled(target:Int, id:Int) {}

	public function cursed(id:Int, level:Int, word:String) {}

	public function curseToken(count:Int) {}

	public function curseScore(excess:Int) {}

	public function badBiomes(id:Int, name:String) {}

	public function vogUpdate() {}

	public function photo(x:Int, y:Int, signature:String) {}

	public function shutdown() {}

	public function global(text:String) {}

	public function war(a:Int, b:Int, status:String) {}

	public function learnedTools(list:Array<Int>) {}

	public function toolExperts(list:Array<Int>) {}

	public function toolSlots(total:Int) {}

	public function babyWiggle(list:Array<Int>) {}

	public function saysLocation(x:Int, y:Int, text:String) {}

	public function dying(id:Int, sick:Bool) {}

	public function says(id:Int, text:String, curse:Bool) {}

	public function emot(id:Int, index:Int, sec:Int) {}

	public function mapChunk(instance:MapInstance) {
		for (y in 0...instance.height) {
			for (x in 0...instance.width) {
				trace(instance.x, x, instance.y, y);
				Render.addObject(x, y, Game.engine.map.object.get(instance.x + x, instance.y + y));
			}
		}
	}

	public function mapChange(change:MapChange) {}

	public function foodChange(store:Int, capacity:Int, ateId:Int, fillMax:Int, speed:Float, responsible:Int) {}

	public function heatChange(heat:Float, foodTime:Float, indoorBonus:Float) {}

	public function frame() {}

	public function lineage(list:Array<Int>, eve:Int) {}

	public function healed(id:Int) {}

	public function monument(x:Int, y:Int, id:Int) {}

	public function grave(x:Int, y:Int, id:Int) {}

	public function graveOld(x:Int, y:Int, pid:Int, poid:Int, age:Float, name:String, lineage:Array<String>) {}

	public function graveMove(xs:Int, ys:Int, xd:Int, yd:Int, swapDest:Bool) {}

	public function ownerList(x:Int, y:Int, list:Array<Int>) {}

	public function valley(spacing:Int, offset:Int) {}

	public function flight(id:Int, x:Int, y:Int) {}

	public function homeland(x:Int, y:Int, name:String) {}

	public function craving(id:Int, bonus:Int) {}

	public function flip(x:Int, y:Int) {}
}
