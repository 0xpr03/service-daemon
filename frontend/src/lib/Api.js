import axios from "axios";

export const AuthState = {
    LOGGED_IN: "LoggedIn",
    PASSWORD: "NotLoggedIn",
    TOTP: "RequiresTOTP",
    SETUP_TOTP: "RequiresTOTPSetup",
};

export const ServiceState = {
    Stopped: "Stopped",
    Running: "Running",
    Ended: "Ended",
    Crashed: "Crashed",
    Stopping: "Stopping",
    Killed: "Killed",
    EndedBackoff: "EndedBackoff",
    CrashedBackoff: "CrashedBackoff",
};

export const ConsoleType = {
    State: "State",
    Stderr: "Stderr",
    Stdout: "Stdout",
    Stdin: "Stdin",
};

export function api_users () {
    return axios.get('/api/user/list');
}

export function api_input (sid, text) {
    // return axios.post('/api/service/' + sid + '/input',
    // {data: text});
    return axios({
        url: '/api/service/' + sid + '/input', method: 'POST', headers: {
            'Content-Type': 'application/json'
        }, data: "\"" + text + "\""
    });
}

export function api_output (sid) {
    return axios.get('/api/service/' + sid + '/output');
}

export function api_stop (sid) {
    return axios.post('/api/service/' + sid + '/stop');
}

export function api_kill (sid) {
    return axios.post('/api/service/' + sid + '/kill');
}

export function api_start (sid) {
    return axios.post('/api/service/' + sid + '/start');
}

export function api_state (service) {
    return axios.get('/api/service/' + service + '/state');
}

export function api_login (email, password) {
    return axios.post('/api/login', {
        email: email,
        password: password
    });
}

export function api_logout () {
    return axios.post('/api/logout');
}

export function api_checklogin () {
    return axios.get('/api/checklogin');
}

export function api_totp (token) {
    return axios({
        url: '/api/totp', method: 'POST', headers: {
            'Content-Type': 'application/json'
        }, data: Number(token)
    });
}

export function api_get_user_info (user) {
    return axios.get("/api/user/" + user + "/info");
}

export function api_set_user_info (user, name, email) {
    return axios.post("/api/user/" + user + "/info", { name: name, email: email });
}

export function api_services_user (user) {
    return axios.get("/api/user/" + user + "/services");
}

/// Get all services for current session
export function api_services () {
    return axios.get("/api/services");
}

export function api_delete_user(user) {
    return axios.post("/api/user/"+user+"/delete");
}

export function api_totp_change(user) {
    return axios.post("/api/user/"+user+"/totp", {});
}

/// Change password for other user without previous password
export function api_password_change_admin(user, password) {
    return axios.post("/api/user/"+user+"/password",{ password: password });
}

/// Change current user password
export function api_password_change(user, password, old_password) {
    return axios.post("/api/user/"+user+"/password",{password: password, old_password: old_password });
}

/// get service permissions of user
export function api_get_perms (user, service) {
    return axios.get("/api/user/" + user + "/permissions/" + service);
}

/// set service permission of user
export function api_set_perms (user, service, perms) {
    return axios.post("/api/user/" + user + "/permissions/" + service, { perms: perms });
}

/// create user
export function api_create_user (name, email, password) {
    return axios.post("/api/user/create", { name: name, email: email, password: password });
}

/// service permissions of current session
export function api_service_permissions (service) {
    return axios.get("/api/service/" + service + "/permissions");
}

/// get latest logs for service
export function api_log_latest (service,amount) {
    return axios.get("/api/service/" + service + "/log/latest/"+amount);
}

/// get log console snapshot
export function api_log_console (service,logid) {
    return axios.get("/api/service/" + service + "/log/console/"+logid);
}

/// get log details
export function api_log_details (service,logid) {
    return axios.get("/api/service/" + service + "/log/details/"+logid);
}

/// global permissions of current session
export function api_global_permissions () {
    return axios.get("/api/");
}

export class Permissions {
    /// Start service
    static START = 1;
    /// Stop service
    static STOP = 2;
    /// Stdin write all
    static STDIN_ALL = 4;
    /// Output inspect
    static OUTPUT = 8;
    /// Kill service
    static KILL = 16;
    /// Inspect service log
    static LOG = 32;

    static hasFlag (input, flag) {
        return (input & flag) != 0;
    };
}

export class Log {
    static ServiceMaxRetries = "ServiceMaxRetries";
    static SystemStart = "SystemStartup";
    static KilledCmd = "ServiceCmdKilled";
    static Killed = "ServiceKilled";
    static StopCmd = "ServiceCmdStop";
    static Ended = "ServiceEnded";
    static Stopped = "ServiceStopped";
    static StartFailure = "ServiceStartFailed"; // (String)
    static Started = "ServiceStarted";
    static StartCmd = "ServiceCmdStart";
    static Crash = "ServiceCrashed"; // (i32)
    static Input = "Stdin"; // string
}

export function formatLog(entry) {
    if (typeof entry.action === 'string') {
        switch (entry.action) {
            case Log.Stopped: return "Service stopped";
            case Log.StopCmd:
                if (entry.invoker)
                    return "Service stop by "+entry.invoker.name;
                else
                    return "Service stop by system";
            case Log.Ended: return "Service ended";
            case Log.SystemStart: return "System startup";
            case Log.Killed: return "Service killed";
            case Log.KilledCmd: return "Service kill by "+ entry.invoker.name;
            case Log.Started: return "Service started";
            case Log.StartCmd:
                if (entry.invoker)
                    return "Service start by "+ entry.invoker.name;
                else 
                    return "Service auto start";
            default: return "Unknown log case: "+entry.action;
        }
    } else {
        switch (Object.keys(entry.action)[0]) {
            case Log.StartFailure: return "Startup failure: "+entry.action[Log.StartFailure];
            case Log.Crash: return "Service crashed, signal "+entry.action[Log.Crash];
            case Log.ServiceMaxRetries: return "Maximum start retries reached: "+entry.action[Log.ServiceMaxRetries];
            case Log.Input: return "Console input by "+entry.invoker.name+": "+entry.action[Log.Input];
        }
    }
    console.error("Unknown log entry!",entry.action);
    return "Unknown log entry! " + entry.action;
}