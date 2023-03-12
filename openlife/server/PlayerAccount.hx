package openlife.server;

import haxe.display.Display.CompletionResult;
import openlife.auto.AiHelper;
import openlife.data.object.ObjectHelper;
import openlife.settings.ServerSettings;
import sys.io.File;

class PlayerAccount {
	public static var AllPlayerAccountsByEmail = new Map<String, PlayerAccount>();
	public static var AllPlayerAccountsById = new Map<Int, PlayerAccount>();
	public static var AccountIdIndex:Int = 1;

	// saved
	public var id:Int;
	public var isAi:Bool = false;

	public var email:String;
	public var account_key_hash:String;
	public var name:String = 'SNOW';
	public var canUseServerCommands = false;

	public var score:Float;
	public var femaleScore:Float;
	public var maleScore:Float;

	public var lastSeenInTicks:Float;

	public var coinsInherited:Float;

	// not saved yet
	public var scoreEntries = new Array<ScoreEntry>(); // is used to store prestige boni / mali
	public var displayClosePlayers:Bool = true;

	// not saved
	public var graves = new Array<ObjectHelper>();

	private function new() {}

	public static function GetOrCreatePlayerAccount(email:String, account_key_hash:String, id:Int = 0):PlayerAccount {
		var account = AllPlayerAccountsByEmail[email];
		if (account != null) return account;

		account = new PlayerAccount();
		account.id = id > 0 ? id : AccountIdIndex++;
		account.email = email;
		account.account_key_hash = account_key_hash;

		AllPlayerAccountsByEmail[account.email] = account;
		AllPlayerAccountsById[account.id] = account;

		// trace('New account: ${id}-->${account.id} $email');

		return account;
	}

	public static function GetPlayerAccountById(id:Int):PlayerAccount {
		return AllPlayerAccountsById[id];
	}

	public static function WritePlayerAccounts(path:String) {
		var accounts = AllPlayerAccountsByEmail;

		// trace('Wrtie to file: $path width: $width height: $height length: $length');

		var writer = File.write(path, true);
		var dataVersion = 1;
		var count = 0;

		for (ac in accounts)
			count++;

		writer.writeInt32(dataVersion);
		writer.writeInt32(count);

		for (ac in accounts) {
			writer.writeInt32(ac.id);
			writer.writeInt8(cast(ac.isAi, Int));

			writer.writeString('${ac.email}\n');
			writer.writeString('${ac.account_key_hash}\n');
			writer.writeString('${ac.name}\n');

			writer.writeFloat(ac.score);
			writer.writeFloat(ac.femaleScore);
			writer.writeFloat(ac.maleScore);

			writer.writeDouble(ac.lastSeenInTicks);

			writer.writeFloat(ac.coinsInherited);
		}

		writer.close();
	}

	public static function ReadPlayerAccounts(path:String) {
		var reader = File.read(path, true);
		var dataVersion = reader.readInt32();
		var count = reader.readInt32();
		AllPlayerAccountsByEmail = new Map<String, PlayerAccount>();
		AllPlayerAccountsById = new Map<Int, PlayerAccount>();

		trace('Read from file: $path count: $count');

		for (i in 0...count) {
			var id = reader.readInt32();
			var isAi = cast(reader.readInt8(), Bool);
			var email = reader.readLine();
			var account_key_hash = reader.readLine();
			var account = GetOrCreatePlayerAccount(email, account_key_hash, id);
			account.isAi = isAi;
			account.name = reader.readLine();
			account.score = reader.readFloat();
			account.femaleScore = reader.readFloat();
			account.maleScore = reader.readFloat();

			account.lastSeenInTicks = reader.readDouble();

			account.coinsInherited = reader.readFloat();
		}

		reader.close();

		// trace('PlayerAccounts: $AllPlayerAccounts');
	}

	public var totalScore(get, null):Float;

	public function get_totalScore() {
		var total = (maleScore + femaleScore) / 2;
		if (this.isAi) total *= ServerSettings.AiTotalScoreFactor;
		total = Math.floor(total);
		return total;
	}

	public static function ChangeScore(player:GlobalPlayerInstance) {
		// TODO give lower score to AI
		var account = player.account;
		var score = player.yum_multiplier;
		var factor = ServerSettings.ScoreFactor;

		account.score = account.score * (1 - factor) + score * factor;

		if (player.isFemale()) account.femaleScore = account.femaleScore * (1 - factor) + score * factor; else
			account.maleScore = account.maleScore * (1 - factor) + score * factor;

		account.score = Math.round(account.score * 100) / 100;
		account.femaleScore = Math.round(account.femaleScore * 100) / 100;
		account.maleScore = Math.round(account.maleScore * 100) / 100;

		trace('Score: ${account.score} This Life: $score femaleScore: ${account.femaleScore} maleScore: ${account.maleScore}');
	}

	public function removeDeletedGraves() {
		for (grave in graves) {
			if (grave.id == 0) graves.remove(grave);
		}
	}

	public function getLastLivingPlayer():GlobalPlayerInstance {
		var account:PlayerAccount = this;

		for (p in GlobalPlayerInstance.AllPlayers) {
			if (p.deleted) continue;
			if (account == p.account) {
				return p;
			}
		}

		return null;
	}

	public function hasCloseBlockingGrave(tx:Int, ty:Int, distance:Float = -1):Bool {
		return calculateCloseBlockingGraveFitness(tx, ty, distance) > 1;
	}

	/**+1 for each grave in a distance of 100 / up to +10 if closer**/
	public function calculateCloseBlockingGraveFitness(tx:Int, ty:Int, distance:Float = -1):Float {
		var playerAccount:PlayerAccount = this;
		var fitness = 0.0;

		if (distance < 0) distance = ServerSettings.GraveBlockingDistance;

		playerAccount.removeDeletedGraves();

		for (grave in playerAccount.graves) {
			if (grave.isBoneGrave() == false) continue;

			var dist = AiHelper.CalculateDistance(tx, ty, grave.tx, grave.ty);
			var tmpFitness = Math.pow(distance, 2) / (1 + dist);
			fitness += tmpFitness > 10 ? 10 : tmpFitness;

			// if(dist > ServerSettings.GraveBlockingDistance * ServerSettings.GraveBlockingDistance) continue;
		}

		// trace('spawnAsEve calculateCloseBlockingGraveFitness: $fitness');

		return fitness;
	}

	public function hasCloseNonBlockingGrave(tx:Int, ty:Int):Bool {
		return calculateCloseNonBlockingGraveFitness(tx, ty) > 1;
	}

	public function calculateCloseNonBlockingGraveFitness(tx:Int, ty:Int):Float {
		var playerAccount:PlayerAccount = this;
		var fitness = 0.0;

		playerAccount.removeDeletedGraves();

		for (grave in playerAccount.graves) {
			if (grave.isGraveWithGraveStone() == false) continue;

			var dist = AiHelper.CalculateDistance(tx, ty, grave.tx, grave.ty);
			var tmpFitness = Math.pow(ServerSettings.GraveBlockingDistance, 2) / (1 + dist);
			fitness += tmpFitness > 10 ? 10 : tmpFitness;
			// if(dist > ServerSettings.GraveBlockingDistance * ServerSettings.GraveBlockingDistance) continue;
		}

		// trace('spawnAsEve calculateCloseNonBlockingGraveFitness: $fitness');

		return fitness;
	}
}
