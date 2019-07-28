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
};

export const LogType = {
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

export function api_password_change(user, password) {
    return axios.post("/api/user/"+user+"/password",{password: password });
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

    static hasFlag (input, flag) {
        return input & flag;
    };
} 