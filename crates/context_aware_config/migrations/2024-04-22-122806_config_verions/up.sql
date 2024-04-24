-- Your SQL goes here
-- Name: functions; Type: TABLE; Schema: public; Owner: -
--
CREATE TABLE public.config_versions (
    id text PRIMARY KEY,
    config json,
    created_at timestamp without time zone DEFAULT CURRENT_TIMESTAMP NOT NULL
);
--