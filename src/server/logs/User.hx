package server.logs;

import sys.db.Types.SString;
import sys.db.Types.SId;
import sys.db.Object;

class User extends Object
{
    public var id:SId;
    public var email:SString<80>;
    public var password:SString<80>;
}