import React from "react";
import { api_services } from "../lib/Api";
import Card from "react-bootstrap/Card";
import CardGroup from "react-bootstrap/CardGroup";
import { NavLink } from "react-router-dom";
import Container from "react-bootstrap/Container";
import Error from "../components/error";
import Loading from "../components/loading";
import { fmtDuration } from "../lib/time";

function Service (props) {
    const service = props.service;
    return (
        <Card>
            <Card.Body>
                <Card.Title as={NavLink} to={"/service/" + service.id}>{service.name}</Card.Title>
                <Card.Text>
                    State: {service.state}<br />
                    Uptime: {fmtDuration(service.uptime)}<br />
                </Card.Text>
            </Card.Body>
        </Card>
    );
}

export default class Overview extends React.Component {
    constructor(props) {
        super(props);
        this.state = {
            services: [],
            error: undefined,
            loading: true,
        };
    }

    setError (err) {
        this.setState({ error: err });
    }
    clearError () {
        this.setError(undefined);
    }
    setLoading (loading) {
        this.setState({ loading });
    }

    componentDidMount () {
        api_services()
            .then(response => {
                this.setState({ services: response.data });
            })
            .catch(error => {
                console.error(error);
                this.setError("Unable to fetch current login: " + error);
            })
            .then(() => this.setLoading(false));
    }

    services () {
        const services = this.state.services;
        return services.map((service) => <Service key={service.id} service={service} />);
    }

    render () {
        if (this.state.loading) {
            return (
                <Loading />
            );
        } else {
            return (
                <Container>
                    <Error error={this.state.error} />
                    <CardGroup>
                        {this.services()}
                    </CardGroup>
                </Container>
            );
        }
    }
}