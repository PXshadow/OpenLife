package openlife.auto;

import haxe.ds.Vector;
import openlife.auto.Pathfinder.Coordinate;
import openlife.data.map.MapData.MapCollision;

class PathfinderNew {
    public static var timePath = 0.1;
    public static var timePathNew = 0.1;
    public static var timePathFound = 0.1;
    public static var timePathFoundNew = 0.1;
    public static var timePathNotFound = 0.1;
    public static var timePathNotFoundNew = 0.1;

    public static function TryDifferentPaths(start:Coordinate, end:Coordinate, map:MapCollision){
        
        var startTime = Sys.time();
        var pathfinder = new Pathfinder(cast map);
        var paths = pathfinder.createPath(start, end, MANHATTAN, true);

        var spentTimeOld = Math.round((Sys.time() - startTime) * 1000); 

        if(spentTimeOld > 10){            
            var startTime = Sys.time();
            paths = pathfinder.createPath(start, end, MANHATTAN, true);
            var spentTimeOld2 = Math.round((Sys.time() - startTime) * 1000); 
            trace('oldVsNew: spentTimeOld: $spentTimeOld --> $spentTimeOld2');
            if(spentTimeOld > spentTimeOld2) spentTimeOld = spentTimeOld2; 
        } 
                    
        var startTime = Sys.time();
        var newPaths = PathfinderNew.CreatePath(start, end, map);

        var spentTimeNew = Math.round((Sys.time() - startTime) * 1000); 

        if(spentTimeNew > 10){
            var startTime = Sys.time();
            newPaths = PathfinderNew.CreatePath(start, end, map);
            
            var spentTimeNew2 = Math.round((Sys.time() - startTime) * 1000); 
            trace('oldVsNew: spentTimeNew: $spentTimeNew --> $spentTimeNew2');
            if(spentTimeNew > spentTimeNew2) spentTimeNew = spentTimeNew2;
        } 

        //var pathZero = paths != null ? '${paths[0]}' : 'NULL';
        var display = (paths != null && newPaths == null) || (paths == null && newPaths != null);
        if (spentTimeOld + spentTimeNew > 20 || display) trace('Pathing: OLD: $end ${paths} t: $spentTimeOld');
        if (spentTimeOld + spentTimeNew > 20 || display) trace('Pathing: NEW: $end ${newPaths} t: $spentTimeNew');

        timePath += spentTimeOld;
        timePathNew += spentTimeNew;
        timePathFound += paths == null ? 0 : spentTimeOld;
        timePathFoundNew += newPaths == null ? 0 : spentTimeNew;
        timePathNotFound += paths != null ? 0 : spentTimeOld;
        timePathNotFoundNew += newPaths != null ? 0 : spentTimeNew;

        if(spentTimeOld + spentTimeNew > 0){
            var oldVsNew = Math.round((timePath / timePathNew) * 100);
            var oldVsNewFound = Math.round((timePathFound / timePathFoundNew) * 100);
            var oldVsNewNotFound = Math.round((timePathNotFound / timePathNotFoundNew) * 100);

            trace('oldVsNew: $oldVsNew oldVsNewFound: $oldVsNewFound oldVsNewNotFound: $oldVsNewNotFound');
            trace('oldVsNew: $spentTimeOld vs $spentTimeNew ${paths != null} vs ${newPaths != null}');
        }
    }
    
	public static function CreatePath(start:Coordinate, dest:Coordinate, map:MapCollision) : Array<Coordinate>{
		var radius = 16;
		var width = radius * 2;
		var currentMap = new Vector<Float>(width * width);
		
		//trace('NewCreatePath: ${start.x},${start.y} --> ${dest.x},${dest.y}');
        var done = CreateDirectPath(start, dest, map, currentMap);
		
		if(done) return CreatePathFromMap(start, dest, currentMap);

		if(done) return null;

		var change = false;
		var done = false;
		var ii = 0;

        // to speed brute force up, calculate some more paths without expecting to find goal
        CreateDirectPath(start, new Coordinate(1,1), map, currentMap);
        CreateDirectPath(start, new Coordinate(1,width - 2), map, currentMap);
        CreateDirectPath(start, new Coordinate(width - 2,1), map, currentMap);
        CreateDirectPath(start, new Coordinate(width - 2, width - 2), map, currentMap);

		var startTime = Sys.time();

		// brute force
		for(i in 0...width){
			if(done) break;
			ii = i;
			for(y in 0...width){
				for(x in 0...width){
					var length = currentMap[Index(x,y)];
					if(length < 1) continue;

					for(py in -1...2){
						for(px in -1...2){
							if(px == 0 && py == 0) continue;
							if(x + px < 0) continue;
							if(y + py < 0) continue;
							if(x + px >= width) continue;
							if(y + py >= width) continue;

							if(currentMap[Index(x + px, y + py)] > 0) continue;
							if(map.isWalkable(x + px, y + py) == false) continue;
							
							var xlength = px == 0 || py == 0 ? 1 : 1.4;
							//trace('NewCreatePath: force: i: $i $x,$y / $px,$py $xlength $length');
							//trace('NewCreatePath: force: i: $i $x,$y / $px,$py $length');

							change = true;
							currentMap[Index(x + px, y + py)] = length + xlength;

							if(x + px == dest.x && y + py == dest.y){
								done = true;
								break;
							}
						}
					}
				}
			}
		}

		var length = currentMap[Index(dest.x,dest.y)];
		var time = Math.round((Sys.time() - startTime) * 1000);

		trace('NewCreatePath: force: done: $done i: $ii ${dest.x},${dest.y} l: $length t: ${time}');

		if(done) return CreatePathFromMap(start, dest, currentMap);

		return null;
	}

	private static function Index(x:Int, y:Int) : Int {
		return x + y * 32;
	}

    public static function CreateDirectPath(start:Coordinate, dest:Coordinate, map:MapCollision, currentMap:Vector<Float>) : Bool {
        var currentX = start.x;
        var currentY = start.y;
        var length = 1.0;
        var radius = 16;
        var width = 32;

        currentMap[Index(currentX,currentY)] = 1;
                
        // direct path
        for(i in 0...radius) {
            length = currentMap[Index(currentX,currentY)];
            //var x = currentX + 1;
            //var y = currentY + 1;
            var px = currentX < dest.x ? 1 : -1; 
            var py = currentY < dest.y ? 1 : -1; 

            if(currentX == dest.x && currentY == dest.y) break;
            if(currentX < 1) break;
            if(currentY < 1) break;
            if(currentX >= width - 1) break;
            if(currentY >= width - 1) break;

            //trace('NewCreatePath: ${currentX},${currentY} --> ${dest.x},${dest.y} $length');

            if(currentX == dest.x && map.isWalkable(currentX, currentY + py)){
                currentY += py;
                length += 1;
                currentMap[Index(currentX,currentY)] = length;
                continue;
            }
            if(currentY == dest.y && map.isWalkable(currentX + px, currentY)){
                currentX += px;
                length += 1;
                currentMap[Index(currentX,currentY)] = length;
                continue;
            }
            if(map.isWalkable(currentX + px, currentY + py)){
                currentX += px;
                currentY += py;
                length += 1.4;
                currentMap[Index(currentX,currentY)] = length;
                continue;
            }
            
            if(currentX == dest.x) {
                if(map.isWalkable(currentX - px, currentY + py)){
                    currentX -= px;
                    currentY += py;
                    length += 1.4;
                    currentMap[Index(currentX,currentY)] = length;
                    continue;
                }
                break;
            }
            
            if(currentY == dest.y) {
                if(map.isWalkable(currentX + px, currentY - py)){
                    currentX += px;
                    currentY -= py;
                    length += 1.4;
                    currentMap[Index(currentX,currentY)] = length;
                    continue;
                }
                break;
            }
            
            if(map.isWalkable(currentX, currentY + py)){
                currentY += py;
                length += 1;
                currentMap[Index(currentX,currentY)] = length;
                continue;
            }
            if(map.isWalkable(currentX + px, currentY)){
                currentX += px;
                length += 1;
                currentMap[Index(currentX,currentY)] = length;
                continue;
            }

            break;
        }

        var done = (currentX == dest.x && currentY == dest.y);
        //trace('NewCreatePath: done: $done ${currentX},${currentY} --> ${dest.x},${dest.y} $length');

        return done;    
    }

	private static function BruteForce() {
		
	}

	private static function CreatePathFromMap(start:Coordinate, dest:Coordinate, currentMap:Vector<Float>) : Array<Coordinate>{
		var maxLength = Math.ceil(currentMap[Index(dest.x,dest.y)]);
		var size = Math.floor(Math.sqrt(currentMap.length));
		var reversePath = new Array<Coordinate>();
		
		var nextX = dest.x;
		var nextY = dest.y;

		for(i in 0...maxLength){
			var currentX = nextX;
			var currentY = nextY;
			var length = currentMap[Index(currentX,currentY)];

			reversePath.push(new Coordinate(currentX - dest.x, currentY - dest.y));

			//trace('CreateReversePath: $currentX,$currentY l: $length');

			if(length == 1) break;

			var bestLength:Float = length;

			for(py in -1...2){
				for(px in -1...2){
					if(px == 0 && py == 0) continue;
					if(currentX + px < 0) continue;
					if(currentY + py < 0) continue;
					if(currentX + px >= size) continue;
					if(currentY + py >= size) continue;

					length = currentMap[Index(currentX + px, currentY + py)];
					
					if(length < 1) continue;
					if(length >= bestLength) continue;
					bestLength = length;
					nextX = currentX + px;
					nextY = currentY + py;
				}
			}
		}

		//trace('Pathing: RP: ${reversePath.toString()} ${reversePath[0].toString()}');
		var path = new Array<Coordinate>();

		while (reversePath.length > 0){
			var c = reversePath.pop();
			//path.push(new Coordinate(c.x + dest.x - start.x, c.y + dest.y - start.y));
			path.push(new Coordinate(c.x + dest.x, c.y + dest.y));
		}

		//trace('Pathing: NP: ${path.toString()}');

		return path;
	}
}