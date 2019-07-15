import React from "react";
import Container from "react-bootstrap/Container";
import ButtonToolbar from "react-bootstrap/ButtonToolbar";
import Row from "react-bootstrap/Row";
import Col from "react-bootstrap/Col";
import Button from "react-bootstrap/Button";
import Error from "../components/error";
import { fmtDuration } from "../lib/time";
import Loading from "../components/loading";
import { Link } from "react-router-dom";
import { api_state, api_start, api_stop, api_kill, api_service_permissions, Permissions, ServiceState } from "../lib/Api";
import { promises } from 'fs';

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
        };

        this.startService = this.startService.bind(this);
        this.stopService = this.stopService.bind(this);
        this.killService = this.killService.bind(this);
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

    refreshState () {
        this.setLoading(true);
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
            .then(() =>
                this.setLoading(false));
    }

    componentDidMount () {
        this.refreshState();
    }

    uptime () {
        const seconds = this.state.uptime;
        return fmtDuration(seconds);
    }

    render () {
        const running = this.state.state === ServiceState.Running;
        const stopping = this.state.state === ServiceState.Stopping;
        const stopped = !running && !stopping;
        const perms = this.state.permissions;

        if (this.state.loading) {
            return (<Loading />);
        } else {
            return (
                <Container>
                    <Row>
                        <Error error={this.state.error} />
                    </Row>
                    <Row>
                        <Col><h2 className="text-secondary">{this.state.name}</h2></Col>
                        <Col><Button disabled={!Permissions.hasFlag(perms, Permissions.OUTPUT)} as={Link} to={"/service/" + this.getSID() + "/console"}>Console</Button></Col>
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
                    <ButtonToolbar>
                        {stopped &&
                            <Col><Button onClick={() => this.startService()}
                                disabled={!Permissions.hasFlag(perms, Permissions.START)} variant="success">Start</Button></Col>
                        }
                        {running &&
                            <Col><Button onClick={() => this.stopService()}
                                disabled={!Permissions.hasFlag(perms, Permissions.STOP)} variant="danger">Stop</Button></Col>
                        }
                        {(running || stopping) &&
                            <Col><Button onClick={() => this.killService()}
                                disabled={!Permissions.hasFlag(perms, Permissions.KILL)} variant="danger">Kill</Button></Col>
                        }
                    </ButtonToolbar>
                    </Row>
                </Container>
            );
        }
    }
}