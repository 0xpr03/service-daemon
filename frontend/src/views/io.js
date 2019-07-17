import React from "react";
import Container from "react-bootstrap/Container";
import Row from "react-bootstrap/Row";
import Output from "../components/output";
import Col from "react-bootstrap/Col";
import Button from "react-bootstrap/Button";
import Error from "../components/error";
import { api_state, api_output, api_input, api_service_permissions, Permissions } from "../lib/Api";
import Form from "react-bootstrap/Form";
import { fmtDuration } from '../lib/time';
import { Link } from "react-router-dom";

export default class IO extends React.Component {
    constructor(props) {
        super(props);

        this.state = {
            output: [],
            error: undefined,
            input: "",
            name: "<Loading>",
            uptime: 0,
            loading: false,
            permissions: props.location.permissions,
        };

        this.handleKeyDown = this.handleKeyDown.bind();
        this.handleChange = this.handleChange.bind();
    }

    handleKeyDown = (e) => {
        if (e.key === 'Enter') {
            console.log("input: " + this.state.input);
            api_input(this.getSID(), this.state.input)
                .then(resp => {
                    this.setState({ input: "" });
                    this.updateOutput();
                    this.clearError();
                })
                .catch(err => {
                    this.setState({ error: "Unable to send input: " + err });
                })

        }
    }

    clearError () {
        this.setState({ error: undefined });
    }

    uptime () {
        return fmtDuration(this.state.uptime);
    }

    getSID () {
        return this.props.match.params.service;
    }

    updatePermissions () {
        return api_service_permissions(this.getSID())
            .then(resp => {
                this.setState({ permissions: resp.data.perms });
            })
            .catch(error => this.setState({ error: "Unable to fetch permissions " + error }));
    }

    updateState () {
        api_state(this.getSID())
            .then(resp => {
                const data = resp.data;
                this.setState({
                    name: data.name, state: data.state, uptime: data.uptime,
                })
            })
            .catch(err => {
                this.setState({ error: "Unable to fetch state: " + err });
            })
    }

    updateOutput () {
        if (Permissions.hasFlag(this.state.permissions, Permissions.OUTPUT)) {
            api_output(this.getSID())
                .then(resp => {
                    this.setState({ output: resp.data });
                    this.clearError();
                })
                .catch(err => {
                    this.setState({ error: "Unable to fetch data: " + err });
                })
        }
    }

    componentDidMount () {
        if (this.state.permissions === undefined) {
            this.updatePermissions()
                .then(() => {
                    this.updateState();
                    this.updateOutput();
                });
        } else {
            this.updateState();
            this.updateOutput();
        }
    }

    handleChange = (e) => {
        this.setState({ input: e.target.value });
    }

    render () {
        const service = this.getSID();
        const perms = this.state.permissions;
        const show_output = Permissions.hasFlag(perms, Permissions.OUTPUT);
        const show_stdin = Permissions.hasFlag(perms, Permissions.STDIN_ALL);
        return (
            <Container fluid={true} className="h-100 pt-md-2">
                <div className="d-flex flex-column h-100">
                    <Row><Error error={this.state.error} /></Row>
                    <Row><Col><h3>{this.state.name}</h3></Col><Col><Button as={Link} to={"/service/" + service}>Back to service</Button></Col></Row>
                    <Row><Col>Uptime: {this.uptime()}</Col></Row>
                    <Row className="d-flex flex-grow-1 flex-fill overflow-auto console-wrapper">
                        {show_output ? (
                            <Output data={this.state.output} />) : (
                                <Col className="text-danger console-col">No permissions for output inspection.</Col>
                            )
                        }
                    </Row>
                    <Row>
                        <Form.Control onChange={this.handleChange}
                            value={this.state.input} placeholder="Enter command.." disabled={!show_stdin} type="text" onKeyDown={this.handleKeyDown} />
                    </Row>
                </div>
            </Container>);
    }
}