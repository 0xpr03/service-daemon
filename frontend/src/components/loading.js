import React from "react";
import { FontAwesomeIcon } from '@fortawesome/react-fontawesome'
import { faSpinner } from '@fortawesome/free-solid-svg-icons'
import { Row, Container, Col, Card } from "react-bootstrap";

export default class Loading extends React.Component {

    render () {
        return (
            <Container className="h-100">
                <Row className="h-100 mx-auto align-items-center">
                    <Col xs={12}>
                        <Card card-block="true">
                            <FontAwesomeIcon className=" align-self-center" icon={faSpinner} size="6x" spin />
                        </Card>
                    </Col>
                </Row>
            </Container>
        );
    }
}