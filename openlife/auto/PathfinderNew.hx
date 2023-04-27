package openlife.auto;

import openlife.settings.ServerSettings;
import sys.io.File;
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

	var map:MapCollision;
	var width:Int;
	var radius:Int;
	var timeOut:Float = 100; // timeout in ms

	var continueFrom:Coordinate = null;

	public var currentMap:Vector<Float> = null;

	var dest:Coordinate = null; // destination

	var changed = false; // used to indicate that the algorithm could still find a new tile
	var changedFromDest = false; // used to indicate that the algorithm could still find a new tile from destination path

	// for debuging
	public var usedBruteForceIterations = 0;

	public function new(newMap:MapCollision) {
		this.map = newMap;
		this.radius = newMap.radius;
		this.width = 2 * radius;
	}

	public static function TryDifferentPaths(start:Coordinate, end:Coordinate, map:MapCollision) {
		var startTime = Sys.time();
		var pathfinder = new Pathfinder(cast map);
		var paths = pathfinder.createPath(start, end, MANHATTAN, true);

		var spentTimeOld = Math.round((Sys.time() - startTime) * 1000);

		if (spentTimeOld > 100) {
			var startTime = Sys.time();
			paths = pathfinder.createPath(start, end, MANHATTAN, true);
			var spentTimeOld2 = Math.round((Sys.time() - startTime) * 1000);
			trace('oldVsNew: spentTimeOld: $spentTimeOld --> $spentTimeOld2');
			if (spentTimeOld > spentTimeOld2) spentTimeOld = spentTimeOld2;
		}

		var newPathfinder = new PathfinderNew(map);
		var startTime = Sys.time();
		var newPaths = newPathfinder.CreatePath(start, end);

		var spentTimeNew = Math.round((Sys.time() - startTime) * 1000);

		if (spentTimeNew > 100) {
			var startTime = Sys.time();
			newPaths = newPathfinder.CreatePath(start, end);

			var spentTimeNew2 = Math.round((Sys.time() - startTime) * 1000);
			trace('oldVsNew: spentTimeNew: $spentTimeNew --> $spentTimeNew2');
			if (spentTimeNew > spentTimeNew2) spentTimeNew = spentTimeNew2;
		}

		// var pathZero = paths != null ? '${paths[0]}' : 'NULL';
		var display = (paths != null && newPaths == null) || (paths == null && newPaths != null);
		if (spentTimeOld + spentTimeNew > 20 || display) trace('Pathing: OLD: $end ${paths} t: $spentTimeOld');
		if (spentTimeOld + spentTimeNew > 20 || display) trace('Pathing: NEW: $end ${newPaths} t: $spentTimeNew');

		timePath += spentTimeOld;
		timePathNew += spentTimeNew;
		timePathFound += paths == null ? 0 : spentTimeOld;
		timePathFoundNew += newPaths == null ? 0 : spentTimeNew;
		timePathNotFound += paths != null ? 0 : spentTimeOld;
		timePathNotFoundNew += newPaths != null ? 0 : spentTimeNew;

		if (spentTimeOld + spentTimeNew > 10) {
			var oldVsNew = Math.round((timePath / timePathNew) * 100);
			var oldVsNewFound = Math.round((timePathFound / timePathFoundNew) * 100);
			var oldVsNewNotFound = Math.round((timePathNotFound / timePathNotFoundNew) * 100);

			trace('oldVsNew: $oldVsNew oldVsNewFound: $oldVsNewFound oldVsNewNotFound: $oldVsNewNotFound');
			trace('oldVsNew: $spentTimeOld vs $spentTimeNew ${paths != null} vs ${newPaths != null}');
		}
	}

	public function WriteMapToFile() {
		var dir = './${ServerSettings.SaveDirectory}/';
		var path = dir + 'paths.txt';
		var writer = File.append(path, false);
		// var writer = File.write(path, false);

		writer.writeString('Destination: ${dest.x},${dest.y}\n');
		for (y in 0...width) {
			var line = '';
			for (x in 0...width) {
				if (x > 0) line += ',';
				line += '${currentMap[Index(x, y)]}';
				// if (currentMap[Index(x, y)] < 0) trace('NewCreatePath: NEGATIVE $x,$y ${currentMap[Index(x, y)]}');
				if (dest.x == x && dest.y == y) line += 'D';
			}

			writer.writeString('$line\n');
		}
		writer.writeString('\n');
		writer.close();
	}

	public function CreatePath(start:Coordinate, dest:Coordinate):Array<Coordinate> {
		this.usedBruteForceIterations = 0;
		this.dest = dest;
		currentMap = new Vector<Float>(width * width);

		// trace('NewCreatePath: ${start.x},${start.y} --> ${dest.x},${dest.y}');
		var done = CreateDirectPath(start, dest, currentMap);

		if (done) return CreatePathFromMap(start, dest, currentMap);

		var crossing = CreatePathBruteForceInCircle(start, dest, currentMap);
		// var crossing = CreatePathBruteForce(start, dest, currentMap);

		if (crossing != null) AddPathFromCrossing(crossing, dest, currentMap);

		if (crossing != null) return CreatePathFromMap(start, dest, currentMap);

		return null;
	}

	private function Index(x:Int, y:Int):Int {
		return x + y * width;
	}

	private function CreatePathBruteForceInCircle(start:Coordinate, dest:Coordinate, currentMap:Vector<Float>) {
		var startTime = Sys.time();
		var crossing:Coordinate = null;

		CreateDirectPath(dest, start, currentMap, -1);

		this.changedFromDest = true;
		this.changed = true;

		// brute force
		for (i in 0...width) {
			if (this.changed == false) break;
			if (this.changedFromDest == false) break;

			this.changedFromDest = false;
			this.changed = false;

			var time = (Sys.time() - startTime) * 1000;
			if (time > timeOut) break;

			this.usedBruteForceIterations = i + 1;

			// start with circle in the middle
			var radius = Math.ceil(width / 2);
			for (rad in 0...radius) {
				var y = rad + start.y;
				for (xx in -rad...rad + 1) {
					var crossing = makePathOneTile(xx + start.x, y);
					if (crossing != null) return crossing;
				}
				var y = -rad + start.y;
				for (xx in -rad...rad + 1) {
					var crossing = makePathOneTile(xx + start.x, y);
					if (crossing != null) return crossing;
				}
				var x = rad + start.x;
				for (yy in -rad + 1...rad) {
					var crossing = makePathOneTile(x, yy + start.y);
					if (crossing != null) return crossing;
				}
				var x = -rad + start.x;
				for (yy in -rad + 1...rad) {
					var crossing = makePathOneTile(x, yy + start.y);
					if (crossing != null) return crossing;
				}
			}
		}

		return null;
	}

	private function makePathOneTile(x:Int, y:Int):Coordinate {
		// trace('makePathOneTile: $x, $y');

		var length = currentMap[Index(x, y)];
		if (length == 0) return null;

		for (py in -1...2) {
			for (px in -1...2) {
				if (px == 0 && py == 0) continue;
				if (x + px < 0) continue;
				if (y + py < 0) continue;
				if (x + px >= width) continue;
				if (y + py >= width) continue;

				var currentLength = currentMap[Index(x + px, y + py)];

				if ((length > 0 && currentLength < 0) || (length < 0 && currentLength > 0)) {
					// trace('NewCreatePath: Force: dest: ${dest.x},${dest.y} crossed path: ${x + px},${y + py} length: $length lengthCurrent: ${currentLength} time: $time');
					return currentLength > 0 ? new Coordinate(x + px, y + py) : new Coordinate(x, y);
				}

				var xlength = px == 0 || py == 0 ? 1 : 1.4;
				var newlength = length > 0 ? length + xlength : length - xlength;

				if (map.isWalkable(x + px, y + py) == false) continue;

				if (length > 0 && currentLength != 0 && currentLength <= newlength) continue;
				if (length < 0 && currentLength != 0 && currentLength >= newlength) continue;

				// trace('NewCreatePath: force: i: $i $x,$y / $px,$py $xlength $length');
				// trace('NewCreatePath: force: i: $i $x,$y / $px,$py $length');

				if (length > 0) this.changed = true; else
					this.changedFromDest = true;

				currentMap[Index(x + px, y + py)] = newlength;
			}
		}

		return null;
	}

	private function CreatePathBruteForce(start:Coordinate, dest:Coordinate, currentMap:Vector<Float>) {
		// to speed brute force up, calculate some more paths without expecting to find goal
		CreateDirectPath(start, new Coordinate(1, 1), currentMap);
		CreateDirectPath(start, new Coordinate(1, width - 2), currentMap);
		CreateDirectPath(start, new Coordinate(width - 2, 1), currentMap);
		CreateDirectPath(start, new Coordinate(width - 2, width - 2), currentMap);

		var startTime = Sys.time();
		var change = true;
		var changeFromDest = true;
		var done = false;
		var crossing:Coordinate = null;

		CreateDirectPath(dest, start, currentMap, -1);

		// TODO brute force is currently only one direction, therefore add other directions
		// brute force
		for (i in 0...width) {
			// if(done) break;
			// if(continueFrom != null) break;
			if (change == false) break;
			if (changeFromDest == false) break;
			change = false;
			changeFromDest = false;

			var time = (Sys.time() - startTime) * 1000;
			if (time > timeOut) break;

			this.usedBruteForceIterations = i + 1;

			for (y in 0...width) {
				for (x in 0...width) {
					var length = currentMap[Index(x, y)];
					if (length == 0) continue;

					for (py in -1...2) {
						for (px in -1...2) {
							if (px == 0 && py == 0) continue;
							if (x + px < 0) continue;
							if (y + py < 0) continue;
							if (x + px >= width) continue;
							if (y + py >= width) continue;

							var currentLength = currentMap[Index(x + px, y + py)];

							if ((length > 0 && currentLength < 0) || (length < 0 && currentLength > 0)) {
								// trace('NewCreatePath: Force: dest: ${dest.x},${dest.y} crossed path: ${x + px},${y + py} length: $length lengthCurrent: ${currentLength} time: $time');
								crossing = currentLength > 0 ? new Coordinate(x + px, y + py) : new Coordinate(x, y);
								done = true;
								return crossing;
							}

							var xlength = px == 0 || py == 0 ? 1 : 1.4;
							var newlength = length > 0 ? length + xlength : length - xlength;

							if (map.isWalkable(x + px, y + py) == false) continue;

							// if(currentLength == 0) continue;
							// if (length > 0 && currentLength > 0) continue;
							// if (length < 0 && currentLength < 0) continue;
							if (length > 0 && currentLength != 0 && currentLength <= newlength) continue;
							if (length < 0 && currentLength != 0 && currentLength >= newlength) continue;

							// trace('NewCreatePath: force: i: $i $x,$y / $px,$py $xlength $length');
							// trace('NewCreatePath: force: i: $i $x,$y / $px,$py $length');

							if (length > 0) change = true; else
								changeFromDest = true;

							// if (length < 0) trace('NewCreatePath: force: NEGATIVE i: $i $x,$y / $px,$py $length');

							currentMap[Index(x + px, y + py)] = newlength;

							/*if(x + px == dest.x && y + py == dest.y){
								done = true;
								break;
							}*/
						}
					}
				}
			}
		}

		var length = currentMap[Index(dest.x, dest.y)];
		var time = Math.round((Sys.time() - startTime) * 1000);
		// trace('NewCreatePath: force: done: $done i: $ii ${dest.x},${dest.y} l: $length t: ${time}');
		// trace('NewCreatePath: force: i: $ii ${dest.x},${dest.y} l: $length t: ${time} cFrom: $change cTo: $changeFromDest');
		return null;
	}

	public function CreateDirectPath(start:Coordinate, dest:Coordinate, currentMap:Vector<Float>, factor:Float = 1):Bool {
		var currentX = start.x;
		var currentY = start.y;
		var length = factor;
		var lengthCurrent:Float = 0;

		currentMap[Index(currentX, currentY)] = length;

		// direct path
		for (i in 0...radius) {
			length = currentMap[Index(currentX, currentY)];
			// var x = currentX + 1;
			// var y = currentY + 1;
			var px = currentX < dest.x ? 1 : -1;
			var py = currentY < dest.y ? 1 : -1;

			if (currentX < 1) break;
			if (currentY < 1) break;
			if (currentX >= width - 1) break;
			if (currentY >= width - 1) break;

			// if creating reverse path check if crossing non reverse path
			if (factor < 0 && lengthCurrent > 0) {
				// trace('NewCreatePath: crossed path: ${currentX},${currentY} length: $length lengthCurrent: ${lengthCurrent}');
				continueFrom = new Coordinate(currentX, currentY);
				currentMap[Index(currentX, currentY)] = lengthCurrent;
				return true;
			}

			if (currentX == dest.x && currentY == dest.y) break;

			// trace('NewCreatePath: ${currentX},${currentY} --> ${dest.x},${dest.y} $length');

			if (currentX == dest.x && map.isWalkable(currentX, currentY + py)) {
				currentY += py;
				length += factor;
				lengthCurrent = currentMap[Index(currentX, currentY)];
				currentMap[Index(currentX, currentY)] = length;
				continue;
			}
			if (currentY == dest.y && map.isWalkable(currentX + px, currentY)) {
				currentX += px;
				length += factor;
				lengthCurrent = currentMap[Index(currentX, currentY)];
				currentMap[Index(currentX, currentY)] = length;
				continue;
			}
			if (map.isWalkable(currentX + px, currentY + py)) {
				currentX += px;
				currentY += py;
				length += 1.4 * factor;
				lengthCurrent = currentMap[Index(currentX, currentY)];
				currentMap[Index(currentX, currentY)] = length;
				continue;
			}

			if (currentX == dest.x) {
				if (map.isWalkable(currentX - px, currentY + py)) {
					currentX -= px;
					currentY += py;
					length += 1.4 * factor;
					lengthCurrent = currentMap[Index(currentX, currentY)];
					currentMap[Index(currentX, currentY)] = length;
					continue;
				}
				break;
			}

			if (currentY == dest.y) {
				if (map.isWalkable(currentX + px, currentY - py)) {
					currentX += px;
					currentY -= py;
					length += 1.4 * factor;
					lengthCurrent = currentMap[Index(currentX, currentY)];
					currentMap[Index(currentX, currentY)] = length;
					continue;
				}
				break;
			}

			if (map.isWalkable(currentX, currentY + py)) {
				currentY += py;
				length += factor;
				lengthCurrent = currentMap[Index(currentX, currentY)];
				currentMap[Index(currentX, currentY)] = length;
				continue;
			}
			if (map.isWalkable(currentX + px, currentY)) {
				currentX += px;
				length += factor;
				lengthCurrent = currentMap[Index(currentX, currentY)];
				currentMap[Index(currentX, currentY)] = length;
				continue;
			}

			break;
		}

		var done = (currentX == dest.x && currentY == dest.y);
		// trace('NewCreatePath: done: $done ${currentX},${currentY} --> ${dest.x},${dest.y} $length');

		return done;
	}

	private function AddPathFromCrossing(crossing:Coordinate, dest:Coordinate, currentMap:Vector<Float>) {
		var nextX = crossing.x;
		var nextY = crossing.y;
		var newLength = currentMap[Index(nextX, nextY)];
		var bestLength:Float = -1000000;

		// from negative to less negativ
		for (i in 0...1000) {
			var currentX = nextX;
			var currentY = nextY;
			var length = currentMap[Index(currentX, currentY)];

			currentMap[Index(currentX, currentY)] = newLength;

			// trace('AddPathFromCrossing: $currentX,$currentY l: $length --> $newLength');
			newLength += 1; // TODO does not consider diagonal

			if (length == -1) break;

			for (py in -1...2) {
				for (px in -1...2) {
					if (px == 0 && py == 0) continue;
					if (currentX + px < 0) continue;
					if (currentY + py < 0) continue;
					if (currentX + px >= width) continue;
					if (currentY + py >= width) continue;

					length = currentMap[Index(currentX + px, currentY + py)];

					if (length > -1) continue;
					if (length <= bestLength) continue;
					bestLength = length;
					nextX = currentX + px;
					nextY = currentY + py;
				}
			}
		}
	}

	private function CreatePathFromMap(start:Coordinate, dest:Coordinate, currentMap:Vector<Float>):Array<Coordinate> {
		var maxLength = Math.ceil(currentMap[Index(dest.x, dest.y)]);
		var reversePath = new Array<Coordinate>();

		var nextX = dest.x;
		var nextY = dest.y;

		for (i in 0...maxLength) {
			var currentX = nextX;
			var currentY = nextY;
			var length = currentMap[Index(currentX, currentY)];

			reversePath.push(new Coordinate(currentX - dest.x, currentY - dest.y));

			// trace('CreateReversePath: $currentX,$currentY l: $length');

			if (length == 1) break;

			var bestLength:Float = length;

			for (py in -1...2) {
				for (px in -1...2) {
					if (px == 0 && py == 0) continue;
					if (currentX + px < 0) continue;
					if (currentY + py < 0) continue;
					if (currentX + px >= width) continue;
					if (currentY + py >= width) continue;

					length = currentMap[Index(currentX + px, currentY + py)];

					if (length < 1) continue;
					if (length >= bestLength) continue;
					bestLength = length;
					nextX = currentX + px;
					nextY = currentY + py;
				}
			}
		}

		// trace('Pathing: RP: ${reversePath.toString()} ${reversePath[0].toString()}');
		var path = new Array<Coordinate>();

		while (reversePath.length > 0) {
			var c = reversePath.pop();
			// path.push(new Coordinate(c.x + dest.x - start.x, c.y + dest.y - start.y));
			path.push(new Coordinate(c.x + dest.x, c.y + dest.y));
		}

		// trace('Pathing: NP: ${path.toString()}');

		return path;
	}
}
