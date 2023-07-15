package openlife.server;

import haxe.Exception;
import openlife.data.object.ObjectData;
import openlife.data.object.ObjectHelper;
import openlife.settings.ServerSettings;
import sys.FileSystem;
import sys.io.File;

using StringTools;

@:enum abstract PrestigeClass(Int) from Int to Int {
	public var NotSet = 0;
	public var Serf = 1;
	public var Commoner = 2;
	public var Noble = 3;
	public var King = 6;
	public var Emperor = 7;
}

// Holds all Saved Lineage  Information
// TODO delete / backup not needed lineages
class Lineage {
	private static var PrestigeClasses = ['Not Set', 'Serf', 'Commoner', 'Noble', 'Noble', 'Noble', 'King', 'Emperor'];

	// sperate in new and old to save faster (use DB???)
	private static var NewLineages = new Map<Int, Lineage>();
	private static var AllLineages = new Map<Int, Lineage>();

	// Statistics
	public static var lastStatisticGenerated:Float = -1;

	public static var reasonKilled = new Map<String, Int>();
	public static var reasonKilledLastDay = new Map<String, Int>();
	public static var reasonKilledLastHour = new Map<String, Int>();
	public static var ages = new Map<Int, Int>();
	public static var agesLastDay = new Map<Int, Int>();
	public static var agesLastHour = new Map<Int, Int>();
	public static var generations = new Map<Int, Int>();

	public static function AddLineage(lineageId:Int, lineage:Lineage) {
		lineage.myId = lineageId;
		NewLineages[lineage.myId] = lineage;
		AllLineages[lineage.myId] = lineage;
	}

	public static function GetLineage(lineageId:Int):Lineage {
		// var lineage = NewLineages[lineageId];
		// if(lineage != null) return lineage;
		return AllLineages[lineageId];
	}

	public var myId:Int = -1;
	public var accountId:Int;

	public var name = ServerSettings.StartingName;

	private var myFamilyName = ServerSettings.StartingFamilyName;

	// use Ids since not all might be available
	public var po_id:Int = -1;
	public var birthTime:Float;
	public var deathTime:Float;
	public var age:Float;
	public var trueAge:Float;
	public var generation = -1; // from dysnasty founder (Eve/Adam)
	public var houseGeneration = -1; // from house founder // not set yet

	public var deathReason:String;
	public var killedByPlayerId:Int; // not set yet
	public var lastSaid:String;
	public var prestige:Float;
	public var coins:Float;
	public var reputation:Float;

	public var myDynastyId:Int = -1; // If a family splits of the old family head (myEveId) is linked with this
	public var myEveId:Int = -1;
	public var motherId:Int = -1;
	public var fatherId:Int = -1;

	public var prestigeClass:PrestigeClass = PrestigeClass.Commoner;
	public var followPlayerId:Int; // not set yet

	public var ownsObject:Bool = false; // indicates if this linage owns a object on the map. If not it might be deleted
	public var alive:Bool = false; // indicates if still alive. If not it might be deleted. Up to now its only set on server restart
	public var delete = false; // mark if linage can be deleted on server restart

	public static function WriteNewLineages(path:String) {
		WriteLineages(path, NewLineages);
	}

	public static function WriteAllLineages(path:String) {
		WriteLineages(path, AllLineages, true);
	}

	// TODO alive and ownsObject is current set only on server start.
	public function canBeDeleted():Bool {
		if (alive) return false;
		if (this.ownsObject) return false; // dont delete if this linage owns still an object on the map like a grave

		var deathSince = this.getDeadSince();
		var trueAge = this.trueAge;
		var delete = deathSince > trueAge * 60 * 24 * ServerSettings.LineageDeleteAgeFactor;

		// trace('canBeDeleted: ${delete} ${name} ${po_id} deathReason: ${deathReason} deathSince: ${Math.ceil(deathSince / (60 * 24))} age: ${Math.ceil(trueAge)}');

		return delete;
	}

	private function setDelete(shouldDelete:Bool) {
		var lineage = this;

		if (this.delete && shouldDelete == false) {
			this.delete = shouldDelete;

			lineage.eveLineage.setDelete(false);
			var mother = lineage.getMotherLineage();
			if (mother != null) mother.setDelete(false);
			var father = lineage.getFatherLineage();
			if (father != null) father.setDelete(false);
		}
		this.delete = shouldDelete;
	}

	// TODO archive important deleted lineages
	public static function WriteLineages(path:String, lineages:Map<Int, Lineage>, deleteOld = false) {
		var count = 0;
		var dataVersion = 3;
		var writer = File.write(path, true);

		if (deleteOld) {
			var countNotDelete = 0;
			var countDelete = 0;

			for (lineage in lineages) {
				lineage.delete = lineage.canBeDeleted();

				// if (lineage.canBeDeleted) countDelete++; else
				//	countNotDelete++;
			}

			// save Eve lineages --> important for family name
			for (lineage in lineages) {
				if (lineage.delete == false) {
					lineage.eveLineage.setDelete(false);
					var mother = lineage.getMotherLineage();
					if (mother != null) mother.setDelete(false);
					var father = lineage.getFatherLineage();
					if (father != null) father.setDelete(false);
				}
			}

			for (lineage in lineages) {
				if (lineage.delete) countDelete++;
				else countNotDelete++;
			}
			count = countNotDelete;
			trace('WriteLineages: canBeDeleted: $countDelete saved: $countNotDelete');
		}
		else {
			for (lineage in lineages)
				count++;

			// trace('WriteLineages: canBeDeleted: NotYetImplemented saved: $count');
		}

		writer.writeInt32(dataVersion);
		writer.writeInt32(count);

		for (lineage in lineages) {
			if (deleteOld && lineage.delete) continue;

			writer.writeInt32(lineage.myId);
			writer.writeInt32(lineage.accountId);

			writer.writeString('${lineage.name}\n');
			writer.writeString('${lineage.familyName}\n'); // writes eve family name

			writer.writeInt32(lineage.po_id);
			writer.writeDouble(lineage.birthTime);
			writer.writeDouble(lineage.deathTime);
			writer.writeFloat(lineage.age);
			writer.writeFloat(lineage.trueAge);

			writer.writeString('${lineage.deathReason}\n');
			writer.writeString('${lineage.lastSaid}\n');
			writer.writeFloat(lineage.prestige);
			writer.writeFloat(lineage.coins);

			writer.writeInt32(lineage.myEveId);
			writer.writeInt32(lineage.motherId);
			writer.writeInt32(lineage.fatherId);

			writer.writeInt8(lineage.prestigeClass);

			// Dataversion 2
			writer.writeInt32(lineage.myDynastyId);
			writer.writeFloat(lineage.reputation);

			// Dataversion 3
			if (lineage.generation < 0) lineage.generation = lineage.calculateGeneration();
			writer.writeInt32(lineage.generation);
			writer.writeInt32(lineage.houseGeneration);
			writer.writeInt32(lineage.killedByPlayerId);
			writer.writeInt32(lineage.followPlayerId);
		}

		writer.close();

		if (ServerSettings.DebugWrite) trace('wrote $count Lineages...');
	}

	public static function ReadAllLineages(pathAll:String, pathNew:String) {
		if (FileSystem.exists(pathAll)) {
			trace('Lineage: exists: $pathAll');
			var pathBackup = pathAll + '.bak';

			try {
				AllLineages = ReadLineages(pathAll);
			} catch (ex) {
				trace('Lineage: restore backup $pathBackup');
				// restore backup
				File.copy(pathBackup, pathAll);
				AllLineages = ReadLineages(pathAll);
			}
			File.copy(pathAll, pathBackup);
		}
		else AllLineages = [];

		var newLineages = ReadLineages(pathNew);

		for (lineage in newLineages)
			AllLineages[lineage.myId] = lineage;

		// WriteAllLineages(pathAll);
	}

	public static function ReadAndSetLineages(path:String):Map<Int, Lineage> {
		AllLineages = ReadLineages(path);
		return AllLineages;
	}

	public static function ReadLineages(path:String):Map<Int, Lineage> {
		var reader = File.read(path, true);
		var supportedDataVersion = 1;
		var dataVersion = reader.readInt32();
		var count = reader.readInt32();
		var loadedLineages = new Map<Int, Lineage>();

		trace('Read lineages from file: $path count: ${count}');

		if (dataVersion < supportedDataVersion)
			throw new Exception('ReadLineages: Data version is: $dataVersion supported data version is: $supportedDataVersion');

		try {
			for (i in 0...count) {
				var lineage = new Lineage(null);

				lineage.myId = reader.readInt32();
				lineage.accountId = reader.readInt32();

				lineage.name = reader.readLine();
				lineage.myFamilyName = reader.readLine();

				lineage.po_id = reader.readInt32();
				lineage.birthTime = reader.readDouble();
				lineage.deathTime = reader.readDouble();
				lineage.age = reader.readFloat();
				lineage.trueAge = reader.readFloat();

				lineage.deathReason = reader.readLine();
				lineage.lastSaid = reader.readLine();
				lineage.prestige = reader.readFloat();
				lineage.coins = reader.readFloat();

				lineage.myEveId = reader.readInt32();
				lineage.motherId = reader.readInt32();
				lineage.fatherId = reader.readInt32();

				lineage.prestigeClass = reader.readInt8();

				// Dataversion 2
				if (dataVersion >= 2) {
					lineage.myDynastyId = reader.readInt32();
					lineage.reputation = reader.readFloat();
				}

				// Dataversion 3
				if (dataVersion >= 3) {
					lineage.generation = reader.readInt32();
					lineage.houseGeneration = reader.readInt32();
					lineage.killedByPlayerId = reader.readInt32();
					lineage.followPlayerId = reader.readInt32();
					// if (lineage.generation < 0) trace('Dataversion 3: ${lineage.generation}');
				}

				loadedLineages[lineage.myId] = lineage;

				// trace('$i read Lineage: ${lineage.myId} FamilyName: ${lineage.myFamilyName} ${lineage.accountId}');

				// if (lineage.myFamilyName.contains('XX')) trace('read Lineage: ${lineage.myId} FamilyName: ${lineage.myFamilyName} ${lineage.accountId}');

				// if (lineage.account == null) trace('WARNING: read Lineage: ${lineage.myId} ${lineage.name} ${lineage.accountId}');
				// trace('read Lineage: ${lineage.myId} ${lineage.name} ${lineage.accountId}');
				// trace('read Lineage: ${lineage.myId} ${lineage.name} ${lineage.account.email}');
			}
		} catch (ex) {
			reader.close();
			throw ex;
		}

		reader.close();

		// trace('read $count Lineages...');

		return loadedLineages;
	}

	public function calculateGeneration() {
		var generation = 0;
		var mother = this.getMotherLineage();

		for (i in 0...10000) {
			if (mother == null) break;
			generation++;
			mother = mother.getMotherLineage();
		}
		return generation;
	}

	public static function GenerateLineageStatistics() {
		var timeSinceLast = TimeHelper.CalculateTimeSinceTicksInSec(lastStatisticGenerated);
		if (lastStatisticGenerated > 0 && timeSinceLast < 60) return false;
		timeSinceLast = TimeHelper.tick;

		var countOld = 0;
		var countNew = 0;
		var reasonKilled = new Map<String, Int>();
		var reasonKilledLastDay = new Map<String, Int>();
		var reasonKilledLastHour = new Map<String, Int>();
		var ages = new Map<Int, Int>();
		var agesLastDay = new Map<Int, Int>();
		var agesLastHour = new Map<Int, Int>();
		var generations = new Map<Int, Int>();

		for (lineage in AllLineages) {
			var yearsSinceBirth = TimeHelper.CalculateTimeSinceTicksInYears(lineage.birthTime);
			var yearsSinceDeath = TimeHelper.CalculateTimeSinceTicksInYears(lineage.deathTime);
			var age = Math.round(yearsSinceBirth - yearsSinceDeath);
			var deathReason = lineage.deathReason == null ? '' : lineage.deathReason;
			var killedBy = deathReason;
			var isLastDay = lineage.deathTime > 0 && yearsSinceDeath < 1440; // 1440 = 24h// 2880 = 48h
			var isLastHour = lineage.deathTime > 0 && yearsSinceDeath < 60;

			if (deathReason.startsWith('reason_killed_')) {
				var idString = deathReason.replace('reason_killed_', '');
				var id = Std.parseInt(idString);
				var objData = ObjectData.getObjectData(id);
				killedBy = objData.name;
			}

			if (isLastDay) countNew++;
			else countOld++;

			var generation = lineage.generation;

			if (generation < 0) {
				generation = lineage.calculateGeneration();
				lineage.generation = generation;
				NewLineages[lineage.myId] = lineage;
			}

			if (age < -1) age = -1;

			ages[age] += 1;
			if (isLastDay) agesLastDay[age] += 1;
			if (isLastHour) agesLastHour[age] += 1;
			reasonKilled[killedBy] += 1;
			if (isLastDay) reasonKilledLastDay[killedBy] += 1;
			if (isLastHour) reasonKilledLastHour[killedBy] += 1;
			generations[generation] += 1;

			// if(lineage.familyName.startsWith('SNOW') == false) trace('Lineage: ${lineage.getFullName()} age: ${age} ${lineage.deathReason}');
			// if(lineage.motherId < 1) trace('Lineage: ${lineage.myEveId} --> ${lineage.getFullName()} age: ${age} ${lineage.deathReason}');
		}

		Lineage.reasonKilled = reasonKilled;
		Lineage.reasonKilledLastDay = reasonKilledLastDay;
		Lineage.reasonKilledLastHour = reasonKilledLastHour;
		Lineage.ages = ages;
		Lineage.agesLastDay = agesLastDay;
		Lineage.agesLastHour = agesLastHour;
		Lineage.generations = generations;

		trace('Lineage: countNew: ${countNew} countOld: ${countOld}');

		return true;
	}

	public static function WriteLineageStatistics() {
		// GenerateLineageStatistics();

		var countOld = 0;
		var countNew = 0;
		var reasonKilled = new Map<String, Int>();
		var ages = new Map<Int, Int>();
		var generations = new Map<Int, Int>();

		var dir = './${ServerSettings.SaveDirectory}/';
		var path = dir + 'PlayerLineages.txt';
		var writer = File.write(path, false);

		for (lineage in AllLineages) {
			var yearsSinceBirth = TimeHelper.CalculateTimeSinceTicksInYears(lineage.birthTime);
			var yearsSinceDeath = TimeHelper.CalculateTimeSinceTicksInYears(lineage.deathTime);
			var age = Math.round(yearsSinceBirth - yearsSinceDeath);
			var deathReason = lineage.deathReason;
			var killedBy = deathReason;

			if (deathReason.startsWith('reason_killed_')) {
				var idString = deathReason.replace('reason_killed_', '');
				var id = Std.parseInt(idString);
				var objData = ObjectData.getObjectData(id);
				killedBy = objData.name;
			}

			if (yearsSinceBirth > 2880) countOld++; // 2880 = 48h
			else countNew++;

			var generation = lineage.generation;

			if (generation < 0) {
				generation = lineage.calculateGeneration();
				lineage.generation = generation;
				NewLineages[lineage.myId] = lineage;
			}

			ages[age] += 1;
			reasonKilled[killedBy] += 1;
			generations[generation] += 1;

			writer.writeString('${lineage.getFullName()} gen: ${generation} age: ${age} ${killedBy}\n');

			// if(lineage.familyName.startsWith('SNOW') == false) trace('Lineage: ${lineage.getFullName()} age: ${age} ${lineage.deathReason}');
			// if(lineage.motherId < 1) trace('Lineage: ${lineage.myEveId} --> ${lineage.getFullName()} age: ${age} ${lineage.deathReason}');
		}

		writer.close();

		var path = dir + 'PlayerDeathReasons.txt';
		var writer = File.write(path, false);

		for (reason in reasonKilled.keys()) {
			writer.writeString('$reason ${reasonKilled[reason]}\n');
			// trace('Lineage: $reason ${reasonKilled[reason]}');
		}

		writer.close();

		var ageList = [for (a in ages.keys()) a];

		ageList.sort(function(a, b) {
			if (a < b) return -1;
			else if (a > b) return 1;
			else return 0;
		});

		var path = dir + 'PlayerAges.txt';
		var writer = File.write(path, false);

		for (age in ageList) {
			writer.writeString('age: $age --> ${ages[age]}\n');
			// trace('Lineage: age: $age --> ${ages[age]}');
		}

		writer.close();

		var generationsList = [for (g in generations.keys()) g];

		generationsList.sort(function(a, b) {
			if (a < b) return -1;
			else if (a > b) return 1;
			else return 0;
		});

		var path = dir + 'PlayerGenerations.txt';
		var writer = File.write(path, false);

		for (generation in generationsList) {
			writer.writeString('generation: $generation --> ${generations[generation]}\n');
			// trace('Lineage: generation: $generation --> ${generations[generation]}');
		}

		writer.close();

		trace('Lineage: countNew: ${countNew} countOld: ${countOld}');
	}

	public function new(player:GlobalPlayerInstance) {
		if (player == null) return;

		this.birthTime = TimeHelper.tick;
		this.myId = player.p_id;
		this.po_id = player.po_id;
		this.accountId = player.account.id;

		// trace('accountId: ${this.accountId}');
	}

	public var className(get, null):String;

	public function get_className() {
		return PrestigeClasses[this.prestigeClass];
	}

	public function getFullName(withUnderscore:Bool = false, ignoreFirstName = false) {
		var isAiText = account.isAi ? ServerSettings.AiNameEnding : '';
		var fullName = ignoreFirstName ? '${this.familyName}$isAiText ${this.className}' : '${this.name} ${this.familyName}$isAiText ${this.className}';
		// var fullName = ignoreFirstName ? '${this.name} ${this.familyName} ${this.className}' : '${this.name} ${this.familyName} ${this.className}';

		if (withUnderscore) return StringTools.replace(fullName, ' ', '_');

		return fullName;
	}

	public function getDeadSince():Int {
		var years = TimeHelper.tick - this.deathTime;
		years *= TimeHelper.tickTime; // seconds
		years /= 60; // years
		return Math.floor(years);
	}

	public var account(get, null):PlayerAccount;

	public function get_account() {
		GlobalPlayerInstance.AcquireMutex();
		var value = PlayerAccount.AllPlayerAccountsById[accountId];
		GlobalPlayerInstance.ReleaseMutex();
		return value;
	}

	public var eve(get, null):GlobalPlayerInstance;

	public function get_eve() {
		GlobalPlayerInstance.AcquireMutex();
		var value = GlobalPlayerInstance.AllPlayerMap[myEveId];
		GlobalPlayerInstance.ReleaseMutex();
		return value;
	}

	public var eveLineage(get, null):Lineage;

	public function get_eveLineage() {
		GlobalPlayerInstance.AcquireMutex();
		var value = AllLineages[myEveId];
		GlobalPlayerInstance.ReleaseMutex();
		return value;
	}

	public var dynastyLneage(get, null):Lineage;

	public function get_dynastyLneage() {
		GlobalPlayerInstance.AcquireMutex();
		var value = AllLineages[myDynastyId];
		GlobalPlayerInstance.ReleaseMutex();
		return value;
	}

	public var mother(get, set):GlobalPlayerInstance;

	public function get_mother() {
		GlobalPlayerInstance.AcquireMutex();
		var value = GlobalPlayerInstance.AllPlayerMap[motherId];
		GlobalPlayerInstance.ReleaseMutex();
		return value;
	}

	public function set_mother(newMother:GlobalPlayerInstance) {
		motherId = newMother.p_id;
		return newMother;
	}

	public var father(get, set):GlobalPlayerInstance;

	public function get_father() {
		GlobalPlayerInstance.AcquireMutex();
		var value = GlobalPlayerInstance.AllPlayerMap[fatherId];
		GlobalPlayerInstance.ReleaseMutex();
		return value;
	}

	public function set_father(newFather:GlobalPlayerInstance) {
		fatherId = newFather.p_id;
		return newFather;
	}

	public function getMotherLineage():Lineage {
		GlobalPlayerInstance.AcquireMutex();
		var value = AllLineages[motherId];
		GlobalPlayerInstance.ReleaseMutex();
		return value;
	}

	public function getFatherLineage():Lineage {
		GlobalPlayerInstance.AcquireMutex();
		var value = AllLineages[fatherId];
		GlobalPlayerInstance.ReleaseMutex();
		return value;
	}

	public var familyName(get, null):String;

	public function get_familyName() {
		return this.eveLineage.myFamilyName;
	}

	// TODO support own family name with ditance X from last and prestiege Y
	public function setFamilyName(newName:String) {
		// FIX: Add to new Lineages so that new name is saved
		NewLineages[this.eveLineage.myId] = this.eveLineage;
		// AllLineages[lineage.myId] = lineage;

		trace('setFamilyName: eve id: ${eveLineage.myId} $familyName ==> $newName');

		return this.eveLineage.myFamilyName = newName;
	}

	// p_id mother_id grandmother_id great_grandmother_id ... eve_id eve=eve_id
	public function createLineageString(withMe:Bool = true):String {
		var lineageString = withMe ? '$myId' : '';

		if (myId == myEveId) return lineageString;

		var tmpMotherLineage = this.getMotherLineage();
		var addedEve = false;

		for (ii in 0...10) {
			if (tmpMotherLineage == null) break;
			if (lineageString.length > 0) lineageString += ' ';
			lineageString += '${tmpMotherLineage.myId}';

			if (tmpMotherLineage.myId == myEveId) {
				addedEve = true;
				break;
			}

			tmpMotherLineage = tmpMotherLineage.getMotherLineage();
		}

		// if lineage too long add "eve_id eve=eve_id"
		if (addedEve == false) {
			lineageString += ' eve_id=$myEveId'; // TODO test
		}

		// trace('Lineage: ${lineageString}');

		return lineageString;
	}

	public var grave(get, null):ObjectHelper;

	public function get_grave() {
		for (grave in account.graves) {
			if (grave.getCreatorId() == myId) return grave;
		}

		return null;
	}

	public function isNobleOrMore() {
		return cast(prestigeClass, Int) >= cast(PrestigeClass.Noble, Int);
	}
}
