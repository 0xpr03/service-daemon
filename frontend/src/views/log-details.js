import React from "react";
import Container from "react-bootstrap/Container";
import Row from "react-bootstrap/Row";
import Output from "../components/output";
import Col from "react-bootstrap/Col";
import Button from "react-bootstrap/Button";
import Loading from "../components/loading";
import Error from "../components/error";
import { api_service_permissions, Permissions, api_log_details, api_log_console, formatLog } from "../lib/Api";
import Form from "react-bootstrap/Form";
import { fmtDuration } from '../lib/time';
import { Link } from "react-router-dom";
import { animateScroll } from "react-scroll";

export default class LogDetails extends React.Component {
    constructor(props) {
        super(props);

        this.state = {
            output: [],
            error: undefined,
            entry: {},
            loading: true,
            console_data: false,
            missing_perm_console: false,
            permissions: props.location.permissions,
        };
    }

    clearError () {
        this.setState({ error: undefined });
    }

    getSID () {
        return this.props.match.params.service;
    }

    getLogID () {
        return this.props.match.params.log
    }

    updatePermissions () {
        return api_service_permissions(this.getSID())
            .then(resp => {
                this.setState({ permissions: resp.data.perms });
            })
            .catch(error => this.setState({ error: "Unable to fetch permissions " + error }));
    }

    updateData () {
        if (Permissions.hasFlag(this.state.permissions, Permissions.LOG)) {
            const sid = this.getSID();
            const logid = this.getLogID();
            api_log_details(sid,logid)
                .then(resp => {
                    console.log("details:",resp);
                    this.setState({ entry: resp.data });
                    this.clearError();
                    if (resp.data.console_log) {
                        let has_perm = Permissions.hasFlag(this.state.permissions, Permissions.OUTPUT);
                        this.setState({missing_perm_console: has_perm});
                        if (has_perm) {
                            api_log_console(sid,logid)
                            .then(resp => {
                                this.setState({output: resp.data, console_data: true});
                            })
                            .catch(err => {
                                this.setState({ error: "Unable to fetch data: " + err });
                            })
                        }
                    } else {
                        this.setState({ console_data: false });
                    }
                })
                .catch(err => {
                    this.setState({ error: "Unable to fetch data: " + err });
                })
                .finally(() => this.setState({ loading: false }));
        } else {
            this.setState({ error: "Missing permissions for service!" });

        }
    }

    componentDidMount () {
        if (this.state.permissions === undefined) {
            this.updatePermissions()
                .then(() => {
                    this.updateData();
                });
        } else {
            this.updateData();
        }
    }

    handleChange = (e) => {
        this.setState({ input: e.target.value });
    }

    render () {
        if (this.state.loading) {
            return (<Loading />);
        } else {
            const service = this.getSID();
            const perms = this.state.permissions;
            const datetime = new Date(this.state.entry.time);
            const console_perm = Permissions.hasFlag(perms, Permissions.OUTPUT);
            return (
                <Container fluid={true} className="h-100 pt-md-2">
                    <div className="d-flex flex-column h-100">
                        <Row>
                            <Col><h3>Log Details</h3></Col>
                            <Col><Button as={Link} to={"/service/" + service}>Back to service</Button></Col>
                        </Row>
                        <Row><Error error={this.state.error} /></Row>
                        <Row><Col>Date</Col><Col>{datetime.toLocaleDateString()}</Col></Row>
                        <Row><Col>Time</Col><Col>{datetime.toLocaleTimeString()}</Col></Row>
                        <Row><Col>Info</Col><Col>{formatLog(this.state.entry)}</Col></Row>
                        <Row id="output" className="d-flex flex-grow-1 flex-fill overflow-auto console-wrapper">
                            {console_perm ? (
                                <Output data={this.state.output} />) : (
                                    <Col className="text-danger console-col">No permissions for output inspection.</Col>
                                )
                            }
                        </Row>
                    </div>
                </Container>);
        }
    }
}