import React from "react";
import Container from "react-bootstrap/Container";
import Row from "react-bootstrap/Row";
import Output from "../components/output";
import Error from "../components/error";
import { api_state, api_output, api_input } from "../lib/Api";
import Form from "react-bootstrap/Form";
import { fmtDuration } from '../lib/time';

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
        };

        this.handleKeyDown = this.handleKeyDown.bind();
        this.handleChange = this.handleChange.bind();
    }

    handleKeyDown = (e) => {
        if (e.key === 'Enter') {
            console.log("input: " + this.state.input);
            api_input(this.getSID(), this.state.input)
            .then(resp => {
                this.setState({input: ""});
                this.updateOutput();
                this.clearError();
            })
            .catch(err => {
                this.setState({ error: "Unable to send input: " + err });
            })
            
        }
    }

    clearError() {
        this.setState({error: undefined});
    }

    uptime () {
        return fmtDuration(this.state.uptime);
    }

    getSID () {
        return this.props.match.params.service;
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
        api_output(this.getSID())
            .then(resp => {
                this.setState({ output: resp.data });
                this.clearError();
            })
            .catch(err => {
                this.setState({ error: "Unable to fetch data: " + err });
            })
    }

    componentDidMount () {
        this.updateState();
        this.updateOutput();
    }

    handleChange = (e) => {
        this.setState({ input: e.target.value });
    }

    render () {

        return (
            <Container fluid={true} className="h-100">
                <div className="d-flex flex-column h-100">
                    <Row><Error error={this.state.error} /></Row>
                    <Row><h3>{this.state.name}</h3></Row>
                    <Row>Uptime: {this.uptime()}</Row>
                    <Row className="d-flex flex-grow-1 flex-fill overflow-auto console-wrapper">
                        <Output data={this.state.output} />
                    </Row>
                    <Row>
                        <Form.Control onChange={this.handleChange}
                            value={this.state.input} type="text" onKeyDown={this.handleKeyDown} />
                    </Row>
                </div>
            </Container>);
    }
}