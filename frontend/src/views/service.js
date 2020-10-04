import React from "react";
import Container from "react-bootstrap/Container";
import Row from "react-bootstrap/Row";
import Col from "react-bootstrap/Col";
import Button from "react-bootstrap/Button";
import Error from "../components/error";
import { fmtDuration } from "../lib/time";
import Loading from "../components/loading";
import { Link } from "react-router-dom";
import { api_state, api_start, api_stop, api_kill, api_service_permissions, Permissions, ServiceState, api_log_latest, formatLog } from "../lib/Api";
import { ButtonGroup } from 'react-bootstrap';

function LogEntry (props) {
    const entry = props.entry;
    console.log(entry);
    const datetime = new Date(entry.time);
    return (
        <Row>
            <Col>{datetime.toLocaleDateString()}</Col>
            <Col>{datetime.toLocaleTimeString()}</Col>
            <Col>{formatLog(entry)}
            {
                entry.console_log && (
                    <Link className="ml1" to={{ pathname: "/service/" + props.service + "/log/"+entry.id }}>Details</Link>
                )
            }</Col>
        </Row>
    );
}

export default class Service extends React.Component {
    constructor(props) {
        super(props);

        this.state = {
            loading: true,
            error: undefined,
            output: "",
            input: "",
            uptime: undefined,
            name: undefined,
            state: undefined,
            restart: false,
            permissions: 0,
            intervalId: undefined,
            log: [],
        };

        this.startService = this.startService.bind(this);
        this.stopService = this.stopService.bind(this);
        this.killService = this.killService.bind(this);
        this.handleBlur = this.handleBlur.bind(this);
        this.handleFocus = this.handleFocus.bind(this);
        this.intervalUpdate = this.intervalUpdate.bind(this);
    }

    startService () {
        console.log("starting..");
        api_start(this.getSID())
            .then(result => {
                this.refreshState();
            })
            .catch(err => {
                this.setState({ error: "Couldn't start service: " + err });
            });
    }

    getSID () {
        return this.props.match.params.service;
    }

    stopService () {
        console.log("stopping..");
        api_stop(this.getSID())
            .then(result => {
                this.refreshState();
            })
            .catch(err => {
                this.setState({ error: "Couldn't stop service: " + err });
            });
    }

    killService () {
        console.log("killing..");
        api_kill(this.getSID())
            .then(result => {
                this.refreshState();
            })
            .catch(err => {
                this.setState({ error: "Couldn't stop service: " + err });
            });
    }

    setLoading (loading) {
        this.setState({ loading });
    }

    refreshState (isInterval) {
        if (!isInterval) { this.setLoading(true); }
        Promise.all([api_state(this.getSID()), api_service_permissions(this.getSID())])
            .then(resp => {
                let data_state = resp[0].data;
                let data_perm = resp[1].data;
                this.setState({
                    name: data_state.name,
                    uptime: data_state.uptime,
                    state: data_state.state,
                    error: undefined,
                    permissions: data_perm.perms,
                });
            })
            .catch(err => {
                console.log(err);
                this.setState({ error: "Unable to fetch data: " + err });
            })
            .then(() => {
                this.getLatestLog();
                if (!isInterval) {
                    this.setLoading(false);
                }
            });
    }

    componentWillUnmount () {
        window.removeEventListener("blur", this.handleBlur);
        window.removeEventListener("focus", this.handleFocus);
        const intervalId = this.state.intervalId;
        if (intervalId !== undefined) {
            clearInterval(this.state.intervalId);
        }
    }

    getLatestLog () {
        if (Permissions.hasFlag(this.state.permissions, Permissions.LOG)) {
            api_log_latest(this.getSID(), 30)
                .then(resp => {
                    console.log('resp data',resp.data);
                    this.setState({ log: resp.data });
                })
                .catch(err => this.setState({ error: "Unable to fetch logs: " + err }));
        }
    }

    intervalUpdate () {
        api_state(this.getSID())
            .then(resp => {
                let data_state = resp.data;
                this.setState({
                    name: data_state.name,
                    uptime: data_state.uptime,
                    state: data_state.state,
                });
            })
            .catch(err => {
                console.log(err);
                this.setState({ error: "Unable to fetch data: " + err });
            });
    }

    handleFocus () {
        console.log("activating..");
        var intervalId = setInterval(this.intervalUpdate, 1000);
        this.setState({ intervalId });
    }

    handleBlur () {
        if (this.state.intervalId !== undefined) {
            console.log("deactivating..");
            clearInterval(this.state.intervalId);
            this.setState({ intervalId: undefined });
        }
    }

    componentDidMount () {
        this.refreshState();
        var intervalId = setInterval(this.intervalUpdate, 1000);
        this.setState({ intervalId });
        window.addEventListener("blur", this.handleBlur, false);
        window.addEventListener("focus", this.handleFocus, false);
    }

    uptime () {
        const seconds = this.state.uptime;
        return fmtDuration(seconds);
    }

    renderLog() {
        const log = this.state.log;
        const service = this.getSID();
        return log.map((entry) => <LogEntry key={entry.unique} entry={entry} service={service} />);
    }

    render () {
        const running = this.state.state === ServiceState.Running;
        const stopping = this.state.state === ServiceState.Stopping;
        const backoff = this.state.state === ServiceState.EndedBackoff || this.state.state === ServiceState.CrashedBackoff;
        const stopped = !running && !stopping && !backoff;
        const perms = this.state.permissions;
        const perm_console = Permissions.hasFlag(perms, Permissions.OUTPUT) || Permissions.hasFlag(perms, Permissions.STDIN_ALL);
        const perm_log = Permissions.hasFlag(perms, Permissions.LOG);

        if (this.state.loading) {
            return (<Loading />);
        } else {
            return (
                <><Container className="pt-md-2">
                    <Row>
                        <Error error={this.state.error} />
                    </Row>
                    <Row>
                        <Col className="col-sm-5"><h2>{this.state.name}</h2></Col>
                        {perm_console ? (
                            <Col className="col-sm-7"><Button as={Link} to={{ pathname: "/service/" + this.getSID() + "/console", permissions: this.state.permissions }}>Console</Button></Col>
                        ) : (null)
                        }
                    </Row>
                    <hr className="divider"></hr>
                    <Row>
                        <Col><mark className="text-info">Uptime:</mark> {this.uptime()}
                        </Col>
                    </Row>
                    <Row>
                        <Col><mark>State:</mark> {this.state.state}</Col>
                    </Row>
                    <Row>
                        <ButtonGroup>
                            {stopped &&
                                <Col><Button onClick={() => this.startService()}
                                    disabled={!Permissions.hasFlag(perms, Permissions.START)} variant="success">Start</Button></Col>
                            }
                            {running &&
                                <Col><Button onClick={() => this.stopService()}
                                    disabled={!Permissions.hasFlag(perms, Permissions.STOP)} variant="danger">Stop</Button></Col>
                            }
                            {backoff &&
                                <Col><Button onClick={() => this.stopService()}
                                    disabled={!Permissions.hasFlag(perms, Permissions.STOP)} variant="danger">Abort Backoff</Button></Col>
                            }
                            {(running || stopping) &&
                                <Col><Button onClick={() => this.killService()}
                                    disabled={!Permissions.hasFlag(perms, Permissions.KILL)} variant="danger">Kill</Button></Col>
                            }
                        </ButtonGroup>
                    </Row>
                </Container>
                { perm_log &&
                    (<Container className="pt-md-2">
                        <h4>Log</h4>
                        {this.renderLog()}
                    </Container>)
                }
                </>
            );
        }
    }
}