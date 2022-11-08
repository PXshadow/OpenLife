package openlife.server;

import openlife.auto.AiHelper;
import openlife.client.ClientTag;
import openlife.server.GlobalPlayerInstance.Emote;
import openlife.settings.ServerSettings;
import sys.io.File;

using StringTools;

class NamingHelper {
	private static var FemaleNames = new Map<String, Map<String, String>>();
	private static var MaleNames = new Map<String, Map<String, String>>();
	private static var FemaleNamesArray = new Array<String>();
	private static var MaleNamesArray = new Array<String>();

	public static function GetRandomName(female:Bool):String {
		var names = female ? FemaleNamesArray : MaleNamesArray;

		return names[WorldMap.calculateRandomInt(names.length - 1)];
	}

	/*
		NM
		p_id first_name last_name
		p_id first_name last_name
		p_id first_name last_name
		...
		p_id first_name last_name
		#


		Gives name of player p_id.

		last_name may be ommitted.
	 */
	public static function DoNaming(p:GlobalPlayerInstance, text:String):String {
		//trace('TEST Naming1: $text');

		var doFamilyName = text.startsWith('I AM');

		if (doFamilyName == false && text.startsWith('YOU ARE') == false) return text;

		var targetPlayer = doFamilyName ? p : p.heldPlayer;

		if (targetPlayer == null) targetPlayer = p.getClosestPlayer(5); // 5

		//trace('TEST Naming2: $text');

		if (targetPlayer == null) return text;

		if (doFamilyName) {
			// TODO allow to change name after some generations and with high prestiege
			if (targetPlayer.familyName != ServerSettings.StartingFamilyName) return text;
		} else if (targetPlayer.name != ServerSettings.StartingName) return text;

		/*
			var strings = text.split(' ');

			if(strings.length < 3) return;

			var name = strings[2]; */

		var nameFromText = GetName(text);

		if (nameFromText.length < 2) return text;

		// var r = ~/^[a-z]+$/i; // only letters
		var r = ~/[^a-z]/i; // true if anything but letters
		if (r.match(nameFromText)) return text; // return if there is anything but letters

		var name = doFamilyName ? nameFromText : GetNameFromList(nameFromText, targetPlayer.isFemale());

		//trace('TEST Naming: $nameFromText ==> $name');

		if (name == null) return text;

		text = text.replace(nameFromText, name);

		if (doFamilyName) {
			// check if name is used
			for (p in GlobalPlayerInstance.AllPlayers) {
				if (p.familyName == name) {
					trace('family name: "$name" is used already!');

					return text;
				}
			}
		
			targetPlayer.lineage.setFamilyName(name); // check if used
		} else {
			// check if name is used
			/*
				for(c in Connection.getConnections())
				{
					if(c.player.name == name && c.player.familyName == p.familyName)
					{
						trace('name: "$name" is used already!');

						return;
					}
				}

				for(ai in Connection.getAis())
				{
					if(ai.player.name == name && ai.player.familyName == p.familyName)
					{
						trace('name: "$name" is used already!');

						return;
					}
			}*/

			targetPlayer.name = name;
			if(targetPlayer.age > 3) targetPlayer.say('${name} ${targetPlayer.familyName}');
		}

		//trace('TEST Naming: ${targetPlayer.p_id} ${targetPlayer.name} ${targetPlayer.familyName}');

		if (doFamilyName) {
			// all family member names changed
			for (p in GlobalPlayerInstance.AllPlayers) {
				trace('FAMILYNAME: ${p.name} ${p.familyName}');

				if (p.familyName == name) {
					p.connection.sendNameToAll();
				}
			}
		} else {
			// only one name changed
			targetPlayer.connection.sendNameToAll();
		}

		p.doEmote(Emote.happy); // dont worry be happy!
		if (p != targetPlayer) targetPlayer.doEmote(Emote.happy);

		return text;
	}

	public static function GetName(text:String, nameFirst:Bool = false):String {
		var strings = text.split(' ');

		if (strings.length < 3) return "";

		var name = nameFirst ? strings[0] : strings[2];

		return name;
	}

	public static function GetPlayerByNameWithMessage(player:GlobalPlayerInstance, name:String):GlobalPlayerInstance {
		var target = NamingHelper.GetPlayerByName(player, name);

		if (target == null || target == player) {
			player.connection.sendGlobalMessage('No one found close enough with the name ${name}!');
			return null;
		}

		return target;
	}

	// TODO support family name???
	public static function GetPlayerByName(player:GlobalPlayerInstance, name:String):GlobalPlayerInstance {
		// trace('Get Player name: $name');

		if (name.length < 2) return null;
		if (name == "YOU") return player.getClosestPlayer(6); // 6

		var bestPlayer = null;
		var bestDistance:Float = 10000; // 100 tiles

		for (p in GlobalPlayerInstance.AllPlayers) {
			// trace('Get Player p name: ${p.name}');

			if (p.name == name) {
				var distance = AiHelper.CalculateDistanceToPlayer(player, p);

				if (distance < bestDistance) {
					bestPlayer = p;
					bestDistance = distance;
				}
			}
		}

		// if(bestPlayer != null) trace('Get Player: Found name: $name is ${bestPlayer.name} ${bestPlayer.familyName}');

		return bestPlayer;
	}

	public static function GetNameFromList(newName:String, female:Bool):String {
		newName = newName.toUpperCase();
		var index = newName.substr(0, 2);
		var letters = "ABCDEFGHIJKLMNOPQRSTUVWXYZ";

		// trace('Name: index: $index');

		for (i in 0...20) {
			// change the name little bit
			if (i > 0) {
				var oldName = newName;
				var newChar = letters.charAt(WorldMap.calculateRandomInt(letters.length - 1));
				index = index.charAt(0) + newChar;

				newName = '${newName.charAt(0)}$newChar${newName.substring(2, newName.length)}';

				trace('NAME: $oldName ==> $newName index: $index');
			}

			var map = female ? FemaleNames[index] : MaleNames[index];

			if (map == null) {
				continue;
			}

			var name = map[newName];

			if (name != null && isUsedName(name) == false) return name; // TODO save to a file if not in list so that they may be added

			for (ii in 1...newName.length - 1) {
				var testName = newName.substr(0, newName.length - ii);

				// if(index == "SU") trace('Name: Test: $testName');

				for (n in map) {
					if (StringTools.startsWith(n, testName) || StringTools.contains(n, testName)) {
						if (isUsedName(name) == false) return n;
					}
				}
			}
		}

		return null;
	}

	private static function isUsedName(name:String):Bool {
		for (p in GlobalPlayerInstance.AllPlayers) {
			if (p.name == name) return true;
		}

		return false;
	}

	public static function ReadNames():Bool {
		var result1 = ReadNamesByGender(true);
		var result2 = ReadNamesByGender(false);

		// Test();

		return result1 && result2;
	}

	public static function ReadNamesByGender(female:Bool):Bool {
		var reader = null;
		var count = 0;

		try {
			// var rtti = haxe.rtti.Rtti.getRtti(ServerSettings);
			var dir = './';
			dir += female ? "femaleNames.txt" : "maleNames.txt";
			var nameMap = female ? FemaleNames : MaleNames;
			var nameArray = female ? FemaleNamesArray : MaleNamesArray;

			reader = File.read(dir, false);

			var name = "";

			while (reader.eof() == false) {
				count++;

				name = reader.readLine();

				var index = name.substr(0, 2);

				var map = nameMap[index];

				if (map == null) {
					map = new Map<String, String>();
					nameMap[index] = map;
				}

				map[name] = name;
				nameArray.push(name);

				// if(StringTools.startsWith(name, 'SU')) trace('Name: $name index: $index $count');
			}
		} catch (ex) {
			if (reader != null) reader.close();

			if ('$ex' == 'Eof') {
				reader.close();
				return true;
			}

			trace(ex);

			return false;
		}

		return true;
	}

	public static function Test() {
		var name = NamingHelper.GetNameFromList('Suoon', false);
		trace('Name: $name');

		var name = NamingHelper.GetNameFromList('Spoon', false);
		trace('Name: $name');

		var name = NamingHelper.GetNameFromList('Spoon', false);
		trace('Name: $name');

		var name = NamingHelper.GetNameFromList('SpoonWood', false);
		trace('Name: $name');

		var name = NamingHelper.GetNameFromList('Filea', false);
		trace('Name: $name');

		var name = NamingHelper.GetNameFromList('natragsing', true);
		trace('Name: $name');

		var name = NamingHelper.GetNameFromList('Martin', true);
		trace('Name: $name');

		var name = NamingHelper.GetNameFromList('Jason', true);
		trace('Name: $name');

		var name = NamingHelper.GetNameFromList('Martina', true);
		trace('Name: $name');

		var name = NamingHelper.GetNameFromList('Alina', true);
		trace('Name: $name');

		var name = NamingHelper.GetNameFromList('xxx', true);
		trace('Name: $name');

		var name = NamingHelper.GetNameFromList('yyyZdsd', true);
		trace('Name: $name');

		var name = NamingHelper.GetNameFromList('yyyZdsd', false);
		trace('Name: $name');
	}
}
