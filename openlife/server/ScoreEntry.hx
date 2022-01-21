package openlife.server;

import openlife.settings.ServerSettings;
import openlife.data.object.ObjectHelper;

class ScoreEntry 
{
    public var accountId:Int;
    public var playerId:Int;
    public var score:Float;
    public var text:String;

    public function new() {
        
    }

    public static function CreateScoreEntryIfGrave(decayedObj:ObjectHelper)
    {
        if(decayedObj.id != 89) return; // Old Grave

        var account = decayedObj.getOwnerAccount();
        var creator = decayedObj.getCreator();

        if(account == null) return;

        var score = new ScoreEntry();
        score.accountId = account.id;
        score.playerId = creator != null ? creator.p_id : 0;
        score.score = -ServerSettings.OldGraveDecayMali;
        score.text = creator != null ? 'No one burried ${creator.name} ${creator.familyName}!' : 'No one burried your old bones!';
        account.scoreEntry.push(score);
    }
}