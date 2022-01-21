package openlife.server;

import openlife.settings.ServerSettings;
import openlife.data.object.ObjectHelper;

class ScoreEntry 
{
    public var accountId:Int;
    public var playerId:Int;
    public var score:Float = 0;
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
        account.scoreEntries.push(score);
    }

    public static function CreateScoreEntryForCursedGrave(cursedGrave:ObjectHelper)
    {
        var account = cursedGrave.getOwnerAccount();
        var creator = cursedGrave.getCreator();
        var creatorId = cursedGrave.getCreatorId();

        if(account == null) return;

        var scoreEntry = null;

        if(creatorId > 0)
        {
            for(entry in account.scoreEntries)
            {
                if(entry.playerId == creatorId)
                {
                    scoreEntry = entry;
                    break;
                }
            }
        }

        if(scoreEntry == null)
        {
            scoreEntry = new ScoreEntry();
            scoreEntry.accountId = account.id;
            scoreEntry.playerId = creator != null ? creator.p_id : 0;
            scoreEntry.text = creator != null ? '${creator.name} ${creator.familyName} bones where cursed!' : 'Your old bones where cursed!';
            account.scoreEntries.push(scoreEntry);
        }
        
        scoreEntry.score -= ServerSettings.CursedGraveMali;
    }

    public static function ProcessScoreEntry(player:GlobalPlayerInstance)
    {
        if(Std.int(player.trueAge) % 5 != 0) return;
        if(player.account.scoreEntries.length < 1) return;

        var scoreEntry = player.account.scoreEntries.shift();
        var score = scoreEntry.score;

        if(score < 0 && player.prestige < 10)
        {
            player.account.scoreEntries.push(scoreEntry);
            return;
        }

        if(score < -20)
        {
            score = -10;
            scoreEntry.score += 10;
            player.account.scoreEntries.push(scoreEntry);
        }

        var message = scoreEntry.score > 0 ? 'You gained ${scoreEntry.score} prestige because ${scoreEntry.text}' : 'You lost ${-scoreEntry.score} prestige because ${scoreEntry.text}';
        
        player.addPrestige(scoreEntry.score);

        player.connection.sendGlobalMessage(message);
    }
}