import React from "react";
import Container from "react-bootstrap/Container";
import Row from "react-bootstrap/Row";
import Col from "react-bootstrap/Col";
import Button from "react-bootstrap/Button";
import Error from "../components/error";
import { fmtDuration } from "../lib/time";
import Loading from "../components/loading";
import { Link } from "react-router-dom";
import { api_state, api_start, api_stop, ServiceState } from "../lib/Api";

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
        };

        this.startService = this.startService.bind(this);
        this.stopService = this.stopService.bind(this);
    }

    startService () {
        console.log("starting..");
        api_start(this.getSID())
            .then(result => {
                this.refreshState();
            })
            .catch(err => {
                this.setState({ error: "Couldn't start service: "+err });
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
                this.setState({ error: "Couldn't stop service: "+err });
            });
    }

    setLoading (loading) {
        this.setState({ loading });
    }

    refreshState () {
        this.setLoading(true);
        api_state(this.getSID())
            .then(resp => {
                let data = resp.data;
                this.setState({
                    name: data.name,
                    uptime: data.uptime,
                    state: data.state,
                    error: undefined,
                });
            })
            .catch(err => {
                this.setState({
                    error: "Unable to load service state! " + err
                });
            })
            .then(() => this.setLoading(false));
    }

    componentDidMount () {
        this.refreshState();
    }

    uptime () {
        const seconds = this.state.uptime;
        return fmtDuration(seconds);
    }

    render () {
        let stopped = this.state.state !== ServiceState.Running;

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
                        <Col><Button as={Link} to={"/service/"+this.getSID()+"/console"}>Console</Button></Col>
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
                        {stopped &&
                            <Col><Button onClick={() => this.startService()} variant="success">Start</Button></Col>
                        }
                        {this.state.state === ServiceState.Running &&
                            <Col><Button onClick={() => this.stopService()} variant="danger">Stop</Button></Col>
                        }
                    </Row>
                </Container>
            );
        }
    }
}