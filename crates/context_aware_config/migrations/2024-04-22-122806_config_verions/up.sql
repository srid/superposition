-- Your SQL goes here
-- Name: functions; Type: TABLE; Schema: public; Owner: -
--
CREATE TABLE public.config_versions (
    id bigint PRIMARY KEY,
    config json NOT NULL,
    config_hash TEXT NOT NULL,
    version_type TEXT NOT NULL CHECK (version_type IN ('STABLE', 'NOISY')),
    created_at timestamp without time zone DEFAULT CURRENT_TIMESTAMP NOT NULL
);
--